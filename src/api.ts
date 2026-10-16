import { invoke } from '@tauri-apps/api/core'
import type { CaptureConfig, CaptureSource, CursorPoint, EditDocument, ExportResult, Project, RecordingStatus } from './types'
import type { Locale, UserPreferences } from './settings'

export const api = {
  sources: () => invoke<CaptureSource[]>('list_sources'),
  projects: () => invoke<Project[]>('list_projects'),
  cursor: (id: string) => invoke<CursorPoint[]>('cursor_track', { id }),
  start: (config: CaptureConfig) => invoke<void>('start_recording', { config }),
  stop: () => invoke<Project>('stop_recording'),
  status: () => invoke<RecordingStatus>('recording_status'),
  save: (id: string, edit: EditDocument) => invoke<void>('save_edit', { id, edit }),
  copy: (path: string) => invoke<void>('copy_file_path', { path }),
  reveal: (path: string) => invoke<void>('reveal_in_folder', { path }),
  preferences: () => invoke<UserPreferences>('get_preferences'),
  savePreferences: (preferences: UserPreferences) => invoke<void>('save_preferences', { preferences }),
  setTrayLanguage: (locale: Locale) => invoke<void>('set_tray_language', { locale }),
  export: (id: string, directory: string | null, locale: Locale) => invoke<ExportResult>('export_project', { id, directory, locale }),
}

export function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error)
}
