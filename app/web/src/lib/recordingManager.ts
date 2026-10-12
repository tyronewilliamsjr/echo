import { createDB, RecoveryContext } from './indexdb'
import { getRecordingHandle, fileNameFromChunk } from './utils'

const S3_PART_TARGET = 8 * 1024 * 1024
// const S3_PART_TARGET = 80000
const MAX_ATTEMPTS = 5

type RecordingStatus =
  | 'idle'
  | 'starting'
  | 'recording'
  | 'paused'
  | 'stopping'
  | 'pending_upload'
  | 'pending_complete'
  | 'finished'
  | 'error'

type UploadStatus =
  | 'idle'
  | 'uploading'
  | 'error'
  | 'success'
  | 'suspended'
  | 'retry_complete'

type UploadTask = {
  part_number: number
  status: UploadStatus
  chunks: { num: number; size: number }[]
  attempts: number
  serverId: string
}

export type RecordingEvent = {
  type:
    | 'recording_error'
    | 'recovery_disabled'
    | 'recovery_enabled'
    | 'recovery_error'
    | 'pending_upload'
    | 'upload_error'
    | 'upload_completed'
    | 'upload_suspended'
  recordingClientId: string
  msg?: string // Message to display to user if error
}

export type RecordingState = {
  status: RecordingStatus
  currChunkNumber: number
  uploadState: {
    status: UploadStatus
    currentPartNumber: number
  }
  recoveryEnabled: boolean
}

const preferredTypes = [
  'audio/webm;codecs=opus',
  'audio/mp4;codecs=mp4a.40.2',
  'audio/mp4',
  'audio/ogg;codecs=opus',
  'audio/webm',
  'audio/ogg',
]

class UploadManager {
  private uploadTasks: UploadTask[]
  private uploadUrls: Map<number, string>
  private uploadStatus: UploadStatus
  private currentPartNumber: number
  private clientId: string
  private recordingDirOpfs: FileSystemDirectoryHandle
  private processingPromise: Promise<void> | null = null
  private recoveryContext: RecoveryContext | null = null
  private mimeType: string

  private notify: () => void
  private emitEvent: (event: RecordingEvent) => void

  constructor(
    clientId: string,
    recordingDirOpfs: FileSystemDirectoryHandle,
    notify: () => void,
    emitEvent: (event: RecordingEvent) => void,
    recoveryContext: RecoveryContext | null,
    mimeType: string,
  ) {
    this.uploadUrls = new Map()
    this.uploadTasks = []
    this.uploadStatus = 'idle'
    this.currentPartNumber = 1
    this.clientId = clientId
    this.recordingDirOpfs = recordingDirOpfs
    this.notify = notify
    this.emitEvent = emitEvent
    this.recoveryContext = recoveryContext
    this.mimeType = mimeType
  }

  getState() {
    return {
      status: this.uploadStatus,
      currentPartNumber: this.currentPartNumber,
    }
  }

  suspend = () => {
    if (this.uploadStatus === 'suspended') return
    this.uploadStatus = 'suspended'
    this.notify()
    this.emitEvent({
      type: 'upload_suspended',
      recordingClientId: this.clientId,
    })
  }

  createUploadTask = async (
    serverId: string,
    chunks: { num: number; size: number }[],
  ) => {
    const partNumber = this.currentPartNumber++
    this.uploadTasks.push({
      part_number: partNumber,
      status: 'idle',
      chunks: chunks,
      attempts: 0,
      serverId: serverId,
    })

    if (this.recoveryContext) {
      await this.recoveryContext.tryRun((store) =>
        store.createUploadTask({
          clientId: this.clientId,
          partNumber: partNumber,
          chunkNumbers: chunks.map((chunk) => chunk.num),
          status: 'pending',
        }),
      )
    }
  }

  async getUploadUrls(serverId: string, parts_numbers: number[]) {
    const data: {
      upload_urls?: {
        part_number: number
        upload_url: string
      }[]
    } = await fetch(`/api/${serverId}/parts`, {
      headers: {
        'Content-Type': 'application/json',
      },
      method: 'POST',
      body: JSON.stringify({ parts_numbers }),
    }).then((response) => response.json())

    if (!data.upload_urls) throw new Error('Failed to get upload urls')
    for (const { part_number, upload_url } of data.upload_urls) {
      this.uploadUrls.set(part_number, upload_url)
    }
    this.uploadStatus = 'idle'
  }

  tryDrain = async () => {
    console.log('draining upload tasks')
    try {
      await this.processTasks()
      if (this.uploadTasks.length > 0 || this.uploadStatus === 'suspended')
        return false

      return true
    } catch (error) {
      return false
    }
  }

  completeUpload = async (serverId: string) => {
    if (this.uploadTasks.length > 0 || this.uploadStatus === 'suspended') {
      throw new Error('Upload tasks are not empty or suspended')
    }

    const response = await fetch(
      `/api/r/${serverId}/complete`,
      {
        method: 'POST',
      },
    )

    if (!response.ok) {
      this.uploadStatus = 'retry_complete'

      throw new Error('Failed to complete upload')
    }
    this.uploadStatus = 'success'
    return true
  }

  processTasks = async () => {
    if (this.processingPromise) return this.processingPromise

    this.processingPromise = this.runTasks().finally(() => {
      this.processingPromise = null
    })

    return this.processingPromise
  }

  private runTasks = async () => {
    if (this.uploadStatus === 'suspended' || this.uploadStatus === 'success')
      return

    if (this.uploadStatus === 'idle') {
      this.uploadStatus = 'uploading'
      this.notify()
    }

    while (this.uploadTasks.length > 0) {
      const task = this.uploadTasks[0]

      try {
        task.status = 'uploading'
        await this.uploadPart(task.serverId, task.part_number, task.chunks)
        this.uploadTasks.splice(this.uploadTasks.indexOf(task), 1)

        const presisted = await this.recoveryContext?.tryRun((store) =>
          store.markUploadTaskCompleted(
            this.clientId,
            task.chunks,
            task.part_number,
          ),
        )

        // Only delete chunks if we persisted the upload task for recovery
        if (presisted) {
          await this.deleteChunks(task.chunks)
        }
      } catch (error) {
        console.error(error)
        task.status = 'error'
        task.attempts++

        if (task.attempts >= MAX_ATTEMPTS) {
          console.log('suspending upload tasks')
          this.suspend()
          break
        }
      }
    }

    if (this.uploadTasks.length === 0 && this.uploadStatus === 'uploading') {
      this.uploadStatus = 'idle'
      this.notify()
    }
  }

  private async deleteChunks(chunks: { num: number; size: number }[]) {
    const results = await Promise.allSettled(
      chunks.map((chunk) =>
        this.recordingDirOpfs.removeEntry(fileNameFromChunk(chunk.num)),
      ),
    )

    for (const result of results) {
      if (result.status === 'rejected') {
        console.warn('Failed to clean up uploaded audio chunk', result.reason)
      }
    }
  }

  private uploadPart = async (
    serverId: string,
    partNumber: number,
    uploadingChunks: { num: number; size: number }[],
  ) => {
    const storage: BlobPart[] = []
    let uploadUrl = ''
    if (!this.uploadUrls.has(partNumber)) {
      console.log('getting upload urls for part number: ', partNumber)
      await this.getUploadUrls(serverId, [
        partNumber,
        partNumber + 1,
        partNumber + 2,
        partNumber + 3,
        partNumber + 4,
        partNumber + 5,
      ])
    }
    uploadUrl = this.uploadUrls.get(partNumber)!

    for (const chunk of uploadingChunks) {
      const fileHandle = await this.recordingDirOpfs.getFileHandle(
        fileNameFromChunk(chunk.num),
      )
      const file = await fileHandle.getFile()
      storage.push(file)
    }

    const audioBlob = new Blob(storage, { type: this.mimeType })
    const response = await new Promise<Response>((res, rej) => {
        fetch(uploadUrl, {
          method: 'PUT',
          body: audioBlob,
        })
          .then(res)
          .catch(rej)
    })
    if (!response.ok) throw new Error('Failed to upload part')
    console.log('part ', partNumber, ' uploaded')
  }
}

class Recording {
  public clientId: string
  public serverId: string | null = null
  public recoveryContext: RecoveryContext | null = null
  private uploadManager: UploadManager
  private status: RecordingStatus = 'idle'
  private mediaRecorder: MediaRecorder
  private currChunkNumber = 0
  private recordingDirOpfs: FileSystemDirectoryHandle
  private pendingChunks: { num: number; size: number }[] = []
  private pendingBytes = 0

  private processData = Promise.resolve()
  private notify: () => void
  private emitEvent: (event: RecordingEvent) => void

  constructor(
    clientId: string,
    recordingDirOpfs: FileSystemDirectoryHandle,
    mediaRecorder: MediaRecorder,
    recoveryContext: RecoveryContext | null,
    notify: () => void,
    emitEvent: (event: RecordingEvent) => void,
  ) {
    this.status = 'idle'
    this.currChunkNumber = 0
    this.mediaRecorder = mediaRecorder
    this.serverId = null
    this.clientId = clientId
    this.recordingDirOpfs = recordingDirOpfs
    this.pendingChunks = []
    this.pendingBytes = 0
    this.notify = notify
    this.emitEvent = emitEvent
    this.recoveryContext = recoveryContext

    this.uploadManager = new UploadManager(
      this.clientId,
      this.recordingDirOpfs,
      notify,
      emitEvent,
      this.recoveryContext,
      this.mediaRecorder.mimeType,
    )
  }

  getState() {
    return {
      status: this.status,
      currChunkNumber: this.currChunkNumber,
      uploadState: this.uploadManager.getState(),
      recoveryEnabled: this.recoveryContext?.isEnabled() ?? false,
    }
  }

  private markPending = () => {
    this.uploadManager.suspend()
    this.status = 'pending_upload'
    this.notify()
    this.emitEvent({
      type: 'pending_upload',
      recordingClientId: this.clientId,
    })
  }

  private createTasks = async () => {
    let selectedBytes = 0
    const chunks = []

    while (selectedBytes < S3_PART_TARGET && this.pendingChunks.length > 0) {
      const chunk = this.pendingChunks.shift()!
      selectedBytes += chunk.size
      chunks.push(chunk)
    }
    this.pendingBytes -= selectedBytes

    // Should not throw errors here, so we can continue recording
    try {
      await this.uploadManager.createUploadTask(this.serverId!, chunks)
    } catch (error) {
      console.error(error)
    }
  }

  private cleanUpChunks = async (maxChunkNumber: number) => {
    return Promise.allSettled(
      Array.from({ length: maxChunkNumber }, (_, index) =>
        this.recordingDirOpfs.removeEntry(fileNameFromChunk(index)),
      ),
    )
  }

  private onDataAvailable = (event: BlobEvent) => {
    if (event.data.size === 0) return
    this.processData = this.processData
      .then(() => this.handleData(event))
      .catch((error) => {
        console.error(error)
        this.status = 'error'
        this.notify()

        this.emitEvent({
          type: 'recording_error',
          recordingClientId: this.clientId,
          msg: 'Failed to save recording data',
        })

        this.mediaRecorder.stop()
        for (const track of this.mediaRecorder.stream.getTracks()) {
          track.stop()
        }
        throw error
      })
  }

  private handleData = async (event: BlobEvent) => {
    console.log('data stored: ', event.data.size)

    const fileHandle = await this.recordingDirOpfs.getFileHandle(
      fileNameFromChunk(this.currChunkNumber),
      { create: true },
    )
    const writable = await fileHandle.createWritable()
    await writable.write(event.data)
    await writable.close()

    if (this.recoveryContext) {
      await this.recoveryContext.tryRun((store) =>
        store.saveChunk({
          clientId: this.clientId,
          chunkNumber: this.currChunkNumber,
          size: event.data.size,
          partNumber: null,
        }),
      )
    }

    this.pendingBytes += event.data.size
    this.pendingChunks.push({
      num: this.currChunkNumber,
      size: event.data.size,
    })

    this.currChunkNumber++

    // No server id yet, so we can't upload
    if (!this.serverId) return

    // Check if we have enough data to upload in background
    if (this.pendingBytes > S3_PART_TARGET) {
      await this.createTasks()
      this.uploadManager.processTasks()
    }
  }

  private handleStop = async () => {
    console.log('Handling the recording stop')
    if (this.status === 'error') return false

    // Wait for all chunks to be stored
    await this.processData

    if (!this.serverId) {
      this.markPending()
      return false
    }

    while (this.pendingBytes > 0) await this.createTasks()

    if (!(await this.uploadManager.tryDrain())) {
      this.markPending()
      return false
    }

    try {
      await this.uploadManager.completeUpload(this.serverId)
    } catch (error) {
      this.status = 'pending_complete'
      this.notify()
      this.emitEvent({
        type: 'upload_error',
        recordingClientId: this.clientId,
        msg: 'Failed to complete upload',
      })

      return false
    }

    this.status = 'finished'
    this.notify()
    this.emitEvent({
      type: 'upload_completed',
      recordingClientId: this.clientId,
    })

    this.pendingChunks = []
    return true
  }

  private initRecordingOnServer = async () => {
    try {
      // Attempt to init recording for server id
      const data = await fetch(`/api/r/init`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          client_id: this.clientId,
          mime_type: this.mediaRecorder.mimeType,
        }),
      }).then((response) => response.json())

      if (!data.server_id) throw new Error('Failed to init recording')
      console.log('Got server id: ', data.server_id)

      this.serverId = data.server_id
      this.notify()
    } catch (error) {
      this.uploadManager.suspend()
      this.notify()
    }
  }
  start = async () => {
    this.mediaRecorder.addEventListener('dataavailable', this.onDataAvailable)
    if (this.recoveryContext) {
      await this.recoveryContext.tryRun((store) =>
        store.createRecording({
          clientId: this.clientId,
          stopped: false,
          createdAt: Date.now(),
          mimeType: this.mediaRecorder.mimeType,
        }),
      )
    }

    this.mediaRecorder.start(1000)
    this.status = 'recording'
    this.notify()

    void this.initRecordingOnServer()
  }

  stop = async () => {
    if (this.status === 'error') return
    const stopPromise = new Promise<boolean>((res, rej) => {
      this.mediaRecorder.addEventListener(
        'stop',
        async () => {
          try {
            res(await this.handleStop())
          } catch (error) {
            this.status = 'error'
            this.notify()
            rej(error)
          }
        },
        { once: true },
      )
    })

    this.status = 'stopping'
    this.notify()
    this.mediaRecorder.stop()
    for (const track of this.mediaRecorder.stream.getTracks()) {
      track.stop()
    }

    if (this.recoveryContext) {
      await this.recoveryContext.tryRun((store) =>
        store.markStopped(this.clientId),
      )
    }

    try {
      const success = await stopPromise

      if (success) {
        await this.cleanUpChunks(this.currChunkNumber)

        if (this.recoveryContext) {
          await this.recoveryContext.tryRun((store) =>
            store.cleanUp(this.clientId),
          )
        }
      }
    } catch (error) {
      console.error(error)
      this.status = 'error'
      this.notify()
    }
  }
}

class RecordingManager {
  private opfsRoot?: FileSystemDirectoryHandle
  private currentRecording?: string
  private recordings: Map<string, Recording> = new Map()
  private snapshot: { recordingState: RecordingState | null } = {
    recordingState: null,
  }
  private isStarting = false

  private listeners = new Set<() => void>()
  private eventsListeners = new Set<(event: RecordingEvent) => void>()
  constructor() {}

  subscribe = (listener: () => void) => {
    this.listeners.add(listener)

    return () => this.listeners.delete(listener)
  }

  private notifyListeners = () => {
    this.listeners.forEach((listener) => listener())
  }

  subscribeToEvents = (listener: (event: RecordingEvent) => void) => {
    this.eventsListeners.add(listener)
    return () => {
      this.eventsListeners.delete(listener)
    }
  }

  private emitEvent = (event: RecordingEvent) => {
    console.log('emitting event: ', event)
    this.eventsListeners.forEach((listener) => listener(event))
  }

  private getOpfsRoot = async () => {
    if (!this.opfsRoot) {
      this.opfsRoot = await navigator.storage.getDirectory()
    }
    return this.opfsRoot
  }

  private createRecoveryStore = async (clientId: string) => {
    const db = await createDB()
    const context = new RecoveryContext(db, () => {
      this.emitEvent({
        type: 'recovery_error',
        recordingClientId: clientId,
        msg: 'Failed to recover recording',
      })
    })
    return context
  }

  getSnapshot = () => {
    return this.snapshot
  }

  private updateSnapshot = () => {
    this.snapshot = {
      recordingState:
        this.recordings.get(this.currentRecording!)?.getState() ?? null,
    }
    this.notifyListeners()
  }

  startRecording = async () => {
    if (this.isStarting || this.currentRecording) return
    this.isStarting = true
    const clientId = crypto.randomUUID()

    const [opfsRootResult, recoveryResult] = await Promise.allSettled([
      this.getOpfsRoot(),
      this.createRecoveryStore(clientId),
    ])

    if (opfsRootResult.status === 'rejected') {
      this.isStarting = false
      throw new Error('Failed to create recording')
    }

    let recoveryContext: RecoveryContext | null = null
    if (recoveryResult.status === 'fulfilled') {
      recoveryContext = recoveryResult.value
    } else {
      this.emitEvent({
        type: 'recovery_disabled',
        recordingClientId: clientId,
        msg: 'Recovery is disabled',
      })
    }

    try {
      const recordingDirOpfs = await getRecordingHandle(
        opfsRootResult.value,
        clientId,
      )

      const mimeType = preferredTypes.find((type) =>
        MediaRecorder.isTypeSupported(type),
      )
      const mediaRecorder = new MediaRecorder(
        await navigator.mediaDevices.getUserMedia({
          audio: true,
        }),
        mimeType ? { mimeType } : undefined,
      )

      const recording = new Recording(
        clientId,
        recordingDirOpfs,
        mediaRecorder,
        recoveryContext,
        this.updateSnapshot,
        this.emitEvent,
      )

      this.recordings.set(clientId, recording)
      this.currentRecording = clientId
      await recording.start()
      this.updateSnapshot()
    } catch (error) {
      console.error(error)
      this.currentRecording = undefined
      this.recordings.delete(clientId)
      this.updateSnapshot()
      this.emitEvent({
        type: 'recording_error',
        recordingClientId: clientId,
        msg: 'Failed to start recording',
      })
      return
    } finally {
      this.isStarting = false
    }
  }

  stopRecording = async () => {
    const recording = this.recordings.get(this.currentRecording!) ?? null
    if (!recording) return

    this.currentRecording = undefined
    this.updateSnapshot()

    await recording.stop()

    if (recording.getState().status === 'finished') {
      this.recordings.delete(recording.clientId)
    }

    this.updateSnapshot()
  }
}

export const recordingManager = new RecordingManager()
