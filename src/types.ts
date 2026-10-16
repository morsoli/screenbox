export type Rect = { x: number; y: number; width: number; height: number }
export type CaptureSource = { id: string; kind: 'screen' | 'window' | 'region'; title: string; width: number; height: number }
export type CaptureConfig = { sourceId: string; kind: CaptureSource['kind']; region: Rect | null; microphone: boolean }
export type Callout = { startMs: number; endMs: number; text: string; x: number; y: number }
export type CursorPoint = { tMs: number; x: number; y: number }
export type EditDocument = {
  trimStartMs: number; trimEndMs: number; crop: Rect; autoZoom: boolean; zoomStrength: number
  padding: number; cursorHighlight: boolean; callouts: Callout[]; preset: 'h264' | 'webm'
}
export type Project = {
  id: string; title: string; originalPath: string; cursorPath: string; durationMs: number
  width: number; height: number; microphone: boolean; createdAt: string
  edit: EditDocument; lastExportPath: string | null
}
export type ExportResult = { videoPath: string; posterPath: string; transcriptPath: string; summaryPath: string; transcriptionWarning: string | null }
export type RecordingStatus = { recording: boolean; elapsedMs: number }
