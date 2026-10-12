export { cn } from 'cn'

export async function getRecordingHandle(
  dirRoot: FileSystemDirectoryHandle,
  clientId: string,
) {
  const recordingHandle = await dirRoot.getDirectoryHandle('recordings', {
    create: true,
  })
  return await recordingHandle.getDirectoryHandle(clientId, { create: true })
}

export function fileNameFromChunk(chunkNumber: number) {
  return chunkNumber.toString().padStart(8, '0')
}
