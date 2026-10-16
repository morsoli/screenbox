export type LanguagePreference = 'system' | 'zh-CN' | 'en'
export type ThemePreference = 'system' | 'light' | 'dark'
export type Locale = 'zh-CN' | 'en'
export type UserPreferences = { language: LanguagePreference; theme: ThemePreference }

export const defaultPreferences: UserPreferences = { language: 'system', theme: 'system' }

export function resolveLanguage(preference: LanguagePreference, systemLanguage: string): Locale {
  if (preference !== 'system') return preference
  return systemLanguage.toLowerCase().startsWith('zh') ? 'zh-CN' : 'en'
}

export function resolveTheme(preference: ThemePreference, systemIsDark: boolean): 'light' | 'dark' {
  if (preference !== 'system') return preference
  return systemIsDark ? 'dark' : 'light'
}

const messages = {
  en: {
    'app.newRecording': 'New recording', 'app.library': 'LIBRARY', 'app.noRecordings': 'Your recordings will appear here.',
    'app.localOnly': 'Local only', 'app.localFiles': 'Files stay on this Mac', 'app.workspace': 'Workspace',
    'app.editRecording': 'Edit recording', 'app.privateWorkspace': 'Private workspace', 'app.retry': 'Retry',
    'app.dismiss': 'Dismiss', 'app.loading': 'Loading your workspace…', 'app.settings': 'Settings',
    'app.language': 'Language', 'app.theme': 'Appearance', 'app.followSystem': 'Follow system',
    'app.simplifiedChinese': '简体中文', 'app.english': 'English', 'app.light': 'Light', 'app.dark': 'Dark',
    'app.allRecordings': 'All recordings', 'app.editFreely': 'Edit freely. Your original recording is never changed.',
    'app.exportVideo': 'Export video', 'app.saving': 'Saving…', 'app.exporting': 'Exporting…',
    'app.preview': 'PREVIEW', 'app.playPreview': 'Play preview', 'app.pausePreview': 'Pause preview',
    'app.previewPosition': 'Preview position', 'app.seconds': 'sec', 'app.trimRange': 'Trim range', 'app.start': 'Start', 'app.end': 'End', 'app.finishFrame': 'Finish the frame',
    'app.autoFraming': 'Automatic framing', 'app.followCursor': 'Follow the cursor smoothly',
    'app.zoomStrength': 'Zoom strength', 'app.padding': 'Padding', 'app.highlightCursor': 'Highlight cursor',
    'app.crop': 'Crop', 'app.left': 'Left', 'app.top': 'Top', 'app.width': 'Width', 'app.height': 'Height',
    'app.resetCrop': 'Reset crop', 'app.textCallouts': 'Text callouts', 'app.newCallout': 'New callout', 'app.addCallout': 'Add callout',
    'app.removeCallout': 'Remove callout {number}', 'app.calloutText': 'Callout {number} text',
    'app.from': 'From', 'app.to': 'To', 'app.outputFormat': 'Output format',
    'app.easyToShare': 'MP4 · easy to share', 'app.smallerFile': 'VP9 · smaller file',
    'app.saveEdit': 'Save edit', 'app.chooseExportFolder': 'Choose a different export folder',
    'app.speechModelDownload': 'Speech model download: {percent}%', 'app.videoReady': 'Your video is ready',
    'app.companionFiles': 'Poster, transcript, and Markdown summary were saved beside it.',
    'app.transcriptNeedsAttention': 'Transcript needs attention: {detail}', 'app.copyFilePath': 'Copy file path',
    'app.revealInFinder': 'Reveal in Finder', 'app.filePathCopied': 'File path copied.',
    'app.createSomethingClear': 'CREATE SOMETHING CLEAR', 'app.recordHeadline': 'Record your screen.',
    'app.recordSubheadline': 'Make every detail count.', 'app.recordDescription': 'Capture a moment, let the frame follow your cursor, and share a polished video from your Mac.',
    'app.chooseWhatToRecord': 'Choose what to record', 'app.selectSource': 'Select a screen, window, or region to begin.',
    'app.noCaptureSources': 'No capture sources available', 'app.allowScreenRecording': 'Allow Screen Recording in macOS Settings, reopen Screenbox, then retry.',
    'app.refreshSources': 'Refresh sources', 'app.recordingSource': 'Recording source',
    'app.fullScreen': 'Full screen', 'app.window': 'Window', 'app.region': 'Region',
    'app.dragRegion': 'Drag to choose an area on this display', 'app.displayArea': '{width}% × {height}% of display',
    'app.microphoneAudio': 'Microphone audio', 'app.addYourVoice': 'Add your voice to the recording',
    'app.stopRecording': 'Stop recording', 'app.starting': 'Starting…', 'app.startRecording': 'Start recording',
    'app.recordingLive': 'Recording is live. Stop from the floating timer or menu bar.',
    'app.recordingSaved': 'Recording saved. The original is safe to edit.',
    'app.recordingsStayLocal': 'Recordings and exports stay on this Mac.',
    'app.cursorFocusHeadline': 'Focus stays where you click.',
    'app.cursorFocusDescription': 'Screenbox follows your cursor with smooth framing. Trim, crop, add callouts, and export when it feels right.',
    'app.recordStep': 'Record', 'app.refineStep': 'Refine', 'app.exportStep': 'Export',
    'app.firstRecording': 'Your first recording starts here',
    'app.firstRecordingHint': 'Choose a source above. macOS may ask for Screen Recording and Microphone access on the first attempt.',
    'app.saveEditSuccess': 'Edit saved. Your original recording is unchanged.',
    'app.exportReady': 'Video and companion files are ready on your Mac.',
    'app.trimInvalid': 'Trim needs at least 0.1 seconds inside the recording.',
    'app.fixTrim': 'Fix the trim range before exporting.',
    'app.chooseSourceFirst': 'Choose a recording source first.', 'app.chooseExportFolderTitle': 'Choose export folder',
    'app.settingsUpdateFailed': 'Could not save your preferences.', 'app.settingsLoadFailed': 'Could not load your preferences.', 'app.progressRender': 'Rendering video locally…',
    'app.progressPoster': 'Creating poster frame…', 'app.progressTranscript': 'Preparing transcript and summary…',
    'app.progressDone': 'Export complete', 'app.savingEdit': 'Saving edit…',
    'error.operationFailed': 'Operation failed', 'error.permission': 'Screen Recording permission is unavailable. Allow Screenbox in System Settings → Privacy & Security, then reopen the app.',
    'error.sourceUnavailable': 'The selected source is no longer available. Choose another source and retry.',
    'error.regionRequired': 'Choose a region before recording.', 'error.alreadyRecording': 'A recording is already in progress.',
    'error.noRecording': 'There is no recording in progress.', 'error.recordingInterrupted': 'Recording stopped unexpectedly. The original file remains in Application Support for recovery.',
    'tray.show': 'Show Screenbox', 'tray.stop': 'Stop recording',
  },
  'zh-CN': {
    'app.newRecording': '新建录制', 'app.library': '录制库', 'app.noRecordings': '录制完成后会显示在这里。',
    'app.localOnly': '仅本机', 'app.localFiles': '文件保存在这台 Mac 上', 'app.workspace': '工作区',
    'app.editRecording': '编辑录制', 'app.privateWorkspace': '本地工作区', 'app.retry': '重试',
    'app.dismiss': '关闭提示', 'app.loading': '正在加载工作区…', 'app.settings': '设置',
    'app.language': '语言', 'app.theme': '外观', 'app.followSystem': '跟随系统',
    'app.simplifiedChinese': '简体中文', 'app.english': 'English', 'app.light': '浅色', 'app.dark': '深色',
    'app.allRecordings': '全部录制', 'app.editFreely': '可随时调整，原始录制不会被修改。',
    'app.exportVideo': '导出视频', 'app.saving': '正在保存…', 'app.exporting': '正在导出…',
    'app.preview': '预览', 'app.playPreview': '播放预览', 'app.pausePreview': '暂停预览',
    'app.previewPosition': '预览位置', 'app.seconds': '秒', 'app.trimRange': '裁剪范围', 'app.start': '开始', 'app.end': '结束', 'app.finishFrame': '调整画面',
    'app.autoFraming': '自动取景', 'app.followCursor': '平滑跟随光标',
    'app.zoomStrength': '缩放强度', 'app.padding': '留白', 'app.highlightCursor': '突出显示光标',
    'app.crop': '裁切', 'app.left': '左侧', 'app.top': '顶部', 'app.width': '宽度', 'app.height': '高度',
    'app.resetCrop': '重置裁切', 'app.textCallouts': '文字标注', 'app.newCallout': '新建标注', 'app.addCallout': '添加标注',
    'app.removeCallout': '删除标注 {number}', 'app.calloutText': '第 {number} 条标注内容',
    'app.from': '起始', 'app.to': '结束', 'app.outputFormat': '导出格式',
    'app.easyToShare': 'MP4 · 方便分享', 'app.smallerFile': 'VP9 · 文件更小',
    'app.saveEdit': '保存编辑', 'app.chooseExportFolder': '选择其他导出目录',
    'app.speechModelDownload': '语音模型下载：{percent}%', 'app.videoReady': '视频已导出',
    'app.companionFiles': '海报、转录文本和 Markdown 摘要已保存到视频旁边。',
    'app.transcriptNeedsAttention': '转录需要处理：{detail}', 'app.copyFilePath': '复制文件路径',
    'app.revealInFinder': '在 Finder 中显示', 'app.filePathCopied': '文件路径已复制。',
    'app.createSomethingClear': '清晰记录每个细节', 'app.recordHeadline': '录下屏幕。',
    'app.recordSubheadline': '让每个细节更清晰。', 'app.recordDescription': '捕捉屏幕内容，让画面平滑跟随光标，轻松制作精致的视频。',
    'app.chooseWhatToRecord': '选择录制内容', 'app.selectSource': '选择屏幕、窗口或区域以开始录制。',
    'app.noCaptureSources': '没有可用的录制来源', 'app.allowScreenRecording': '请在 macOS 设置中允许屏幕录制，重新打开 Screenbox 后重试。',
    'app.refreshSources': '刷新来源', 'app.recordingSource': '录制来源',
    'app.fullScreen': '整个屏幕', 'app.window': '窗口', 'app.region': '区域',
    'app.dragRegion': '在此显示器上拖动选择录制区域', 'app.displayArea': '占屏幕 {width}% × {height}%',
    'app.microphoneAudio': '麦克风音频', 'app.addYourVoice': '将人声录入视频',
    'app.stopRecording': '停止录制', 'app.starting': '正在启动…', 'app.startRecording': '开始录制',
    'app.recordingLive': '正在录制。可通过悬浮计时器或菜单栏停止。',
    'app.recordingSaved': '录制已保存，原始文件可随时恢复。',
    'app.recordingsStayLocal': '录制和导出文件都保存在本机。',
    'app.cursorFocusHeadline': '画面跟随你的操作。',
    'app.cursorFocusDescription': 'Screenbox 会平滑跟随光标。你可以裁剪画面、添加文字标注，再导出视频。',
    'app.recordStep': '录制', 'app.refineStep': '编辑', 'app.exportStep': '导出',
    'app.firstRecording': '从第一次录制开始',
    'app.firstRecordingHint': '选择上方的录制来源。首次录制时，macOS 可能会请求屏幕录制和麦克风权限。',
    'app.saveEditSuccess': '编辑已保存，原始录制保持不变。',
    'app.exportReady': '视频及配套文件已保存到这台 Mac。',
    'app.trimInvalid': '裁剪范围必须在录制时长内，且至少保留 0.1 秒。',
    'app.fixTrim': '请先修正裁剪范围再导出。',
    'app.chooseSourceFirst': '请先选择录制来源。', 'app.chooseExportFolderTitle': '选择导出目录',
    'app.settingsUpdateFailed': '无法保存设置。', 'app.settingsLoadFailed': '无法读取设置。', 'app.progressRender': '正在本地渲染视频…',
    'app.progressPoster': '正在生成海报帧…', 'app.progressTranscript': '正在准备转录和摘要…',
    'app.progressDone': '导出完成', 'app.savingEdit': '正在保存编辑…',
    'error.operationFailed': '操作失败', 'error.permission': '无法使用屏幕录制权限。请在「系统设置 → 隐私与安全性」中允许 Screenbox，然后重新打开应用。',
    'error.sourceUnavailable': '所选来源已不可用。请选择其他来源后重试。',
    'error.regionRequired': '请先选择录制区域。', 'error.alreadyRecording': '当前已有录制正在进行。',
    'error.noRecording': '当前没有正在进行的录制。', 'error.recordingInterrupted': '录制意外停止。原始文件仍保存在 Application Support 中，可供恢复。',
    'tray.show': '显示 Screenbox', 'tray.stop': '停止录制',
  },
} as const

export type MessageKey = keyof typeof messages.en

export function translate(locale: Locale, key: MessageKey, values: Record<string, string | number> = {}): string {
  const template: string = messages[locale][key] ?? messages.en[key]
  return template.replace(/\{(\w+)\}/g, (_, name: string) => String(values[name] ?? `{${name}}`))
}

const knownErrors: Record<string, MessageKey> = {
  'Screen Recording permission is unavailable. Allow Screenbox in System Settings → Privacy & Security, then reopen the app.': 'error.permission',
  'Selected source is no longer available': 'error.sourceUnavailable',
  'Choose a region before recording': 'error.regionRequired',
  'A recording is already in progress': 'error.alreadyRecording',
  'No recording in progress': 'error.noRecording',
  'Recording stopped unexpectedly. The original file remains in Application Support for recovery.': 'error.recordingInterrupted',
}

export function localizedError(error: unknown, locale: Locale): string {
  const raw = error instanceof Error ? error.message : String(error)
  const key = knownErrors[raw]
  return key ? translate(locale, key) : `${translate(locale, 'error.operationFailed')}: ${raw}`
}
