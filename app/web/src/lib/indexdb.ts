const DB_NAME = 'recording-manager'
const DB_VERSION = 2

const STORES = {
  recordings: 'recording',
  chunks: 'chunks',
  uploads: 'upload',
} as const

function deleteByClientId(store: IDBObjectStore, clientId: string) {
  const index = store.index('clientId')
  const request = index.openKeyCursor(IDBKeyRange.only(clientId))

  request.onsuccess = () => {
    const cursor = request.result
    if (!cursor) return

    store.delete(cursor.primaryKey)
    cursor.continue()
  }
}

export interface IDB {
  createRecording: (recording: StoreRecording) => Promise<void>
  markStopped: (clientId: string) => Promise<void>
  saveChunk: (chunk: StoredChunk) => Promise<void>
  createUploadTask: (task: StoredUploadTask) => Promise<void>
  markUploadTaskCompleted: (
    clientId: string,
    chunks: { num: number }[],
    partNumber: number,
  ) => Promise<void>

  cleanUp: (clientId: string) => Promise<void>
}

type StoreRecording = {
  clientId: string
  stopped: boolean
  createdAt: number
  mimeType: string
}

type StoredChunk = {
  clientId: string
  chunkNumber: number
  size: number
  partNumber: number | null
}

type StoredUploadTask = {
  clientId: string
  partNumber: number
  chunkNumbers: number[]
  status: 'pending' | 'completed' | 'error'
}

export type RecoveryStore = IDBDatabase & IDB

function getServerState(
  clientId: string,
  recoveryId: string,
): Promise<{
  id: string
  parts: { part_number: number; size: number; e_tag: string }[]
}> {
  return fetch(
    `/api/r/state?client_id=${clientId}&recovery_id=${recoveryId}`,
  ).then((res) => res.json())
}

function runOperation<T>(request: IDBRequest<T>): Promise<T> {
  return new Promise((res, rej) => {
    request.onsuccess = () => {
      res(request.result)
    }
    request.onerror = () => {
      rej(new Error('Failed to run operation'))
    }
  })
}

function getLocalState(store: RecoveryStore, clientId: string) {
  return new Promise<{
    uploadState: StoredUploadTask[]
    chunksState: StoredChunk[]
  }>((res, rej) => {
    const tx = store.transaction([STORES.chunks, STORES.uploads], 'readonly')
    const uploadStateRequest = runOperation<StoredUploadTask[]>(
      tx.objectStore(STORES.uploads).getAll(IDBKeyRange.only(clientId)),
    )
    const chunksStateRequest = runOperation<StoredChunk[]>(
      tx.objectStore(STORES.chunks).getAll(IDBKeyRange.only(clientId)),
    )

    Promise.all([uploadStateRequest, chunksStateRequest])
      .then(([uploadState, chunksState]) => {
        res({
          uploadState,
          chunksState,
        })
      })
      .catch(rej)
  })
}

export class RecoveryContext {
  private store: RecoveryStore
  private enabled = true
  private onError: (msg: string) => void

  constructor(store: RecoveryStore, onError: (msg: string) => void) {
    this.store = store
    this.enabled = true
    this.onError = onError
  }

  isEnabled = () => {
    return this.enabled
  }

  tryRun = async (operation: (store: RecoveryStore) => Promise<void>) => {
    if (!this.enabled) return false

    try {
      await operation(this.store)
      return true
    } catch (error) {
      console.error(error)
      this.enabled = false
      this.onError('Error running recovery operation')
      return false
    }
  }
}

export async function createDB() {
  const indexedDB = globalThis.indexedDB

  const db = await new Promise<RecoveryStore>((resolve, reject) => {
    const request: IDBOpenDBRequest = indexedDB.open(DB_NAME, DB_VERSION)
    request.onupgradeneeded = function (event: IDBVersionChangeEvent) {
      const result = request.result

      const oldVersion = event.oldVersion
      // const newVersion = event.newVersion

      if (oldVersion < 2) {
        if (!result.objectStoreNames.contains(STORES.recordings)) {
          result.createObjectStore(STORES.recordings, {
            keyPath: 'clientId',
          })
        }

        if (!result.objectStoreNames.contains(STORES.chunks)) {
          const store = result.createObjectStore(STORES.chunks, {
            keyPath: ['clientId', 'chunkNumber'],
          })

          store.createIndex('clientId', 'clientId')
          store.createIndex('part', ['clientId', 'partNumber'])
        }

        if (!result.objectStoreNames.contains(STORES.uploads)) {
          const store = result.createObjectStore(STORES.uploads, {
            keyPath: ['clientId', 'partNumber'],
          })

          store.createIndex('clientId', 'clientId')
        }
      }
    }

    request.onsuccess = function (event: Event) {
      const results = (event.target as IDBOpenDBRequest).result

      results.onversionchange = () => {
        results.close()
      }

      resolve(results as RecoveryStore)
    }

    request.onerror = function () {
      reject(new Error('Failed to create database'))
    }

    request.onblocked = () => {
      console.warn('IndexedDB upgrade is blocked by another open connection')
    }
  })

  db.createRecording = (recording: StoreRecording) => {
    return new Promise((res, rej) => {
      const tx = db.transaction(STORES.recordings, 'readwrite')
      tx.objectStore(STORES.recordings).add(recording)

      tx.oncomplete = () => res(void 0)
      tx.onerror = () => rej(new Error('Failed to create recording'))
      tx.onabort = () => rej(new Error('Failed to create recording'))
    })
  }

  db.markStopped = async (clientId: string) => {
    return new Promise((res, rej) => {
      const tx = db.transaction(STORES.recordings, 'readwrite')
      const objectStore = tx.objectStore(STORES.recordings)
      const request: IDBRequest<StoreRecording> = objectStore.get(clientId)

      request.onsuccess = (event) => {
        const recording = (
          event.target as IDBRequest<StoreRecording | undefined>
        ).result

        if (!recording) {
          tx.abort()
          return
        }

        recording.stopped = true
        objectStore.put(recording)
      }
      request.onerror = () => tx.abort()

      tx.oncomplete = () => res(void 0)
      tx.onabort = () => rej(new Error('Failed to mark recording as stopped'))
      tx.onerror = () => rej(new Error('Failed to mark recording as stopped'))
    })
  }

  db.saveChunk = (chunk: StoredChunk) => {
    return new Promise((res, rej) => {
      const tx = db.transaction(STORES.chunks, 'readwrite')
      const store = tx.objectStore(STORES.chunks)
      store.put(chunk)

      tx.oncomplete = () => {
        res(void 0)
      }
      tx.onerror = () => rej(new Error('Failed to save chunk'))
      tx.onabort = () => rej(new Error('Failed to save chunk'))
    })
  }

  db.createUploadTask = async (task: StoredUploadTask) => {
    return new Promise((res, rej) => {
      const tx = db.transaction([STORES.uploads, STORES.chunks], 'readwrite')
      const uploadStore = tx.objectStore(STORES.uploads)
      const chunksStore = tx.objectStore(STORES.chunks)
      uploadStore.put(task)

      for (const chunkNumber of task.chunkNumbers) {
        const req = chunksStore.get([task.clientId, chunkNumber])
        req.onsuccess = () => {
          const chunk = req.result
          if (!chunk) {
            tx.abort()
            return
          }
          chunk.partNumber = task.partNumber
          chunksStore.put(chunk)
        }
        req.onerror = () => tx.abort()
      }

      tx.oncomplete = () => res(void 0)
      tx.onerror = () => rej(new Error('Failed to create upload task'))
      tx.onabort = () => rej(new Error('Failed to create upload task'))
    })
  }

  db.markUploadTaskCompleted = async (
    clientId: string,
    chunks: { num: number }[],
    partNumber: number,
  ) => {
    return new Promise((res, rej) => {
      const tx = db.transaction([STORES.uploads, STORES.chunks], 'readwrite')

      const taskStore = tx.objectStore(STORES.uploads)
      const chunkStore = tx.objectStore(STORES.chunks)
      const taskRequest = taskStore.get([clientId, partNumber])
      taskRequest.onsuccess = () => {
        const task = taskRequest.result as StoredUploadTask | undefined
        if (!task) {
          tx.abort()
          return
        }
        task.status = 'completed'
        taskStore.put(task)
      }
      taskRequest.onerror = () => tx.abort()

      for (const chunk of chunks) {
        const chunkRequest = chunkStore.get([clientId, chunk.num])
        chunkRequest.onsuccess = () => {
          const storedChunk = chunkRequest.result as StoredChunk | undefined
          if (!storedChunk) {
            tx.abort()
            return
          }
          storedChunk.partNumber = partNumber
          chunkStore.put(storedChunk)
        }
      }

      tx.oncomplete = () => res(void 0)
      tx.onerror = () => rej(new Error('Failed to complete upload task'))
      tx.onabort = () => rej(new Error('Failed to complete upload task'))
    })
  }

  db.cleanUp = async (clientId: string) => {
    return new Promise((res, rej) => {
      const tx = db.transaction(
        [STORES.recordings, STORES.chunks, STORES.uploads],
        'readwrite',
      )

      tx.objectStore(STORES.recordings).delete(clientId)
      deleteByClientId(tx.objectStore(STORES.chunks), clientId)
      deleteByClientId(tx.objectStore(STORES.uploads), clientId)

      tx.oncomplete = () => res(void 0)
      tx.onerror = () => rej(new Error('Failed to clean up'))
      tx.onabort = () => rej(new Error('Failed to clean up'))
    })
  }

  return db
}
