import {
  useEffect,
  useSyncExternalStore,
} from 'react'
import type { RecordingEvent } from '@/lib/recordingManager'
import { createFileRoute } from '@tanstack/react-router'
import { recordingManager } from '@/lib/recordingManager'

export const Route = createFileRoute('/_authed/dashboard/r/new/')({
  component: RouteComponent,
})

function useMediaRecorder() {
  const state = useSyncExternalStore(
    recordingManager.subscribe,
    recordingManager.getSnapshot,
    () => recordingManager.getSnapshot(),
  )

  return {
    ...state,
    startRecording: recordingManager.startRecording,
    stopRecording: recordingManager.stopRecording,
  }
}

function useRecordingEvents() {
  useEffect(() => {
    return recordingManager.subscribeToEvents((event: RecordingEvent) => {
      switch (event.type) {
        case 'recording_error':
          console.error('recording error', event.msg)
          break
        case 'upload_error':
          console.error('upload error', event.msg)
          break
        case 'upload_completed':
          console.log('upload completed')
          break
        case 'upload_suspended':
          console.error('upload suspended')
          break
        default:
          console.error('unknown recording event', event)
          break
      }
    })
  }, [])

  return null
}

function RouteComponent() {
  const { recordingState, startRecording, stopRecording } = useMediaRecorder()
  useRecordingEvents()
  // console.log('recording uploadState ', recordingState?.uploadState)

  return (
    <div>
      Hello "/_authed/dashboard/recordings/new/"!
      <div>
        <button
          className="bg-blue-500 text-white p-2 rounded-md hover:bg-blue-600"
          onClick={startRecording}
        >
          Start Recording
        </button>

        <button
          className="bg-red-500 text-white p-2 rounded-md hover:bg-red-600 block mt-4"
          onClick={stopRecording}
        >
          Stop Recording
        </button>

        <button
          className="bg-green-500 text-white p-2 rounded-md hover:bg-green-600 block mt-4"
          onClick={() => {
            console.log(
              'recording currChunkNumber ',
              recordingState?.currChunkNumber,
            )
            console.log('recording status ', recordingState?.status)
            console.log('recording uploadState ', recordingState?.uploadState)
          }}
        >
          console status
        </button>
      </div>
    </div>
  )
}
