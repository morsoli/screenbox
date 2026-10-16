import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState } from 'react'
import type { PointerEvent } from 'react'
import { convertFileSrc } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'
import { ArrowLeft, Check, ChevronDown, Circle, Clapperboard, Copy, FolderOpen, LoaderCircle, Mic, Monitor, MousePointer2, Pause, Play, Plus, Scissors, Settings2, Sparkles, Square, Video, WandSparkles, X } from 'lucide-react'
import { api } from './api'
import { clamp, clock, dateLabel, validTrim } from './format'
import type { Callout, CaptureSource, CursorPoint, EditDocument, ExportResult, Project, Rect } from './types'
import { defaultPreferences, localizedError, resolveLanguage, resolveTheme, translate } from './settings'
import type { Locale, MessageKey, UserPreferences } from './settings'
import './App.css'

const isOverlay = new URLSearchParams(window.location.search).has('overlay')
const fullRect: Rect = { x: 0, y: 0, width: 1, height: 1 }

type AppError = { key?: MessageKey; reason?: unknown; detail?: unknown }
type UiContextValue = {
  locale: Locale
  theme: 'light' | 'dark'
  preferences: UserPreferences
  preferencesReady: boolean
  settingsError: AppError | null
  updatePreferences: (next: UserPreferences) => Promise<void>
}
const errorText = (error: AppError, locale: Locale) => error.key
  ? `${translate(locale, error.key)}${error.detail ? ` ${localizedError(error.detail, locale)}` : ''}`
  : localizedError(error.reason, locale)
const UiContext = createContext<UiContextValue | null>(null)
function useUi(): UiContextValue {
  const value = useContext(UiContext)
  if (!value) throw new Error('User interface context is unavailable')
  return value
}
const sourceLabel = (kind: CaptureSource['kind'], locale: Locale) => translate(locale, kind === 'screen' ? 'app.fullScreen' : kind === 'window' ? 'app.window' : 'app.region')
const progressMessage = (stage: string): MessageKey => {
  const keys: Record<string, MessageKey> = { render: 'app.progressRender', poster: 'app.progressPoster', transcript: 'app.progressTranscript', done: 'app.progressDone' }
  return keys[stage] ?? 'app.progressRender'
}

function TimerOverlay() {
  const { locale } = useUi()
  const [elapsed, setElapsed] = useState(0)
  useEffect(() => {
    const timer = window.setInterval(() => { api.status().then(status => setElapsed(status.elapsedMs)).catch(() => {}) }, 500)
    return () => window.clearInterval(timer)
  }, [])
  return <div className="timer-overlay"><span className="live-dot" /><strong>{clock(elapsed)}</strong><button aria-label={translate(locale, 'app.stopRecording')} onClick={() => void api.stop()}><Square size={14} fill="currentColor" /></button></div>
}

function RegionPicker({ value, onChange, locale }: { value: Rect; onChange: (next: Rect) => void; locale: Locale }) {
  const ref = useRef<HTMLDivElement>(null)
  const anchor = useRef<{ x: number; y: number } | null>(null)
  const locate = (event: PointerEvent<HTMLDivElement>) => {
    const box = ref.current!.getBoundingClientRect()
    return { x: clamp((event.clientX-box.left)/box.width,0,1), y: clamp((event.clientY-box.top)/box.height,0,1) }
  }
  return <div className="region-control"><div className="region-surface" ref={ref}
    onPointerDown={event => { anchor.current = locate(event); event.currentTarget.setPointerCapture(event.pointerId) }}
    onPointerMove={event => { if (!anchor.current) return; const point = locate(event); onChange({ x: Math.min(anchor.current.x,point.x), y: Math.min(anchor.current.y,point.y), width: Math.max(.03,Math.abs(anchor.current.x-point.x)), height: Math.max(.03,Math.abs(anchor.current.y-point.y)) }) }}
    onPointerUp={() => { anchor.current = null }}>
      <div className="region-selection" style={{ left: `${value.x*100}%`, top: `${value.y*100}%`, width: `${value.width*100}%`, height: `${value.height*100}%` }} />
      <span>{translate(locale, 'app.dragRegion')}</span>
    </div><small>{translate(locale, 'app.displayArea', { width: Math.round(value.width*100), height: Math.round(value.height*100) })}</small></div>
}

function SettingsMenu() {
  const { locale, preferences, updatePreferences } = useUi()
  const t = (key: MessageKey) => translate(locale, key)
  return <details className="settings-menu">
    <summary aria-label={t('app.settings')}><Settings2 size={16} /><span>{t('app.settings')}</span></summary>
    <div className="settings-popover">
      <fieldset className="settings-group"><legend>{t('app.language')}</legend><div className="settings-choices" role="group" aria-label={t('app.language')}>
        {([['system', 'app.followSystem'], ['zh-CN', 'app.simplifiedChinese'], ['en', 'app.english']] as const).map(([value, label]) => <button type="button" aria-pressed={preferences.language === value} data-testid={`language-${value}`} key={value} onClick={() => void updatePreferences({ ...preferences, language: value })}>{t(label)}</button>)}
      </div></fieldset>
      <fieldset className="settings-group"><legend>{t('app.theme')}</legend><div className="settings-choices" role="group" aria-label={t('app.theme')}>
        {([['system', 'app.followSystem'], ['light', 'app.light'], ['dark', 'app.dark']] as const).map(([value, label]) => <button type="button" aria-pressed={preferences.theme === value} data-testid={`theme-${value}`} key={value} onClick={() => void updatePreferences({ ...preferences, theme: value })}>{t(label)}</button>)}
      </div></fieldset>
    </div>
  </details>
}

function AppMain() {
  const { locale, preferences, updatePreferences, settingsError } = useUi()
  const t = (key: MessageKey, values: Record<string, string | number> = {}) => translate(locale, key, values)
  const fail = (reason: unknown) => setError({ reason })
  const clearError = () => setError(null)
  const [sources, setSources] = useState<CaptureSource[]>([])
  const [projects, setProjects] = useState<Project[]>([])
  const [selectedSource, setSelectedSource] = useState('')
  const [selectedProject, setSelectedProject] = useState<string | null>(null)
  const [region, setRegion] = useState<Rect>({ x: .15, y: .15, width: .7, height: .7 })
  const [microphone, setMicrophone] = useState(true)
  const [recording, setRecording] = useState(false)
  const [elapsed, setElapsed] = useState(0)
  const [busy, setBusy] = useState<'loading' | 'recording' | 'saving' | 'exporting' | null>('loading')
  const [error, setError] = useState<AppError | null>(null)
  const [notice, setNotice] = useState<MessageKey | ''>('')
  const [edit, setEdit] = useState<EditDocument | null>(null)
  const [exported, setExported] = useState<ExportResult | null>(null)
  const [exportStage, setExportStage] = useState<MessageKey>('app.savingEdit')
  const [modelPercent, setModelPercent] = useState<number | null>(null)
  const videoRef = useRef<HTMLVideoElement>(null)
  const [playing, setPlaying] = useState(false)
  const [previewTime, setPreviewTime] = useState(0)
  const [cursorTrack, setCursorTrack] = useState<CursorPoint[]>([])
  const selected = useMemo(() => sources.find(source => `${source.kind}:${source.id}` === selectedSource), [sources,selectedSource])
  const project = useMemo(() => projects.find(item => item.id === selectedProject), [projects,selectedProject])
  const cursorPoint = cursorTrack.findLast(point => point.tMs <= previewTime)
  const activeCallout = edit?.callouts.find(item => item.startMs <= previewTime && item.endMs > previewTime)
  const currentError = error ? errorText(error, locale) : settingsError ? errorText(settingsError, locale) : ''

  const refresh = useCallback(async () => {
    const [sourcesResult,projectsResult,statusResult] = await Promise.allSettled([api.sources(),api.projects(),api.status()])
    if (sourcesResult.status === 'fulfilled') {
      const nextSources = sourcesResult.value
      setSources(nextSources)
      setSelectedSource(current => current || (nextSources[0] ? `${nextSources[0].kind}:${nextSources[0].id}` : ''))
    } else fail(sourcesResult.reason)
    if (projectsResult.status === 'fulfilled') setProjects(projectsResult.value)
    else fail(projectsResult.reason)
    if (statusResult.status === 'fulfilled') { setRecording(statusResult.value.recording); setElapsed(statusResult.value.elapsedMs) }
  }, [locale])
  useEffect(() => {
    refresh().catch(fail).finally(() => setBusy(null))
    const timer = window.setInterval(() => { api.status().then(status => { setRecording(status.recording); setElapsed(status.elapsedMs) }).catch(() => {}) }, 1000)
    const subscriptions = Promise.all([
      listen<Project>('recording-stopped', event => { setRecording(false); setSelectedProject(event.payload.id); setEdit(event.payload.edit); setNotice('app.recordingSaved'); void refresh() }),
      listen<{ stage: string }>('export-progress', event => setExportStage(progressMessage(event.payload.stage))),
      listen<{ percent: number }>('model-download', event => setModelPercent(event.payload.percent)),
    ])
    return () => { window.clearInterval(timer); void subscriptions.then(items => items.forEach(unlisten => unlisten())) }
  }, [refresh, locale])
  useEffect(() => { if (project) { setEdit(structuredClone(project.edit)); setExported(null); clearError(); setNotice('') } }, [project?.id])
  useEffect(() => { if (project) { api.cursor(project.id).then(setCursorTrack).catch(() => setCursorTrack([])) } else setCursorTrack([]) }, [project?.id])

  async function start() {
    if (!selected) { setError({ key: 'app.chooseSourceFirst' }); return }
    setBusy('recording'); clearError(); setNotice('')
    try {
      await api.start({ sourceId: selected.id, kind: selected.kind, region: selected.kind === 'region' ? region : null, microphone })
      setRecording(true); setElapsed(0); setNotice('app.recordingLive')
    } catch (reason) { fail(reason) } finally { setBusy(null) }
  }
  async function stop() {
    setBusy('recording'); clearError()
    try { const result = await api.stop(); setSelectedProject(result.id); setEdit(result.edit); await refresh() }
    catch (reason) { fail(reason) } finally { setBusy(null) }
  }
  async function save() {
    if (!project || !edit) return
    if (!validTrim(edit.trimStartMs,edit.trimEndMs,project.durationMs)) { setError({ key: 'app.trimInvalid' }); return }
    setBusy('saving'); clearError()
    try { await api.save(project.id,edit); setProjects(current => current.map(item => item.id === project.id ? { ...item, edit } : item)); setNotice('app.saveEditSuccess') }
    catch (reason) { fail(reason) } finally { setBusy(null) }
  }
  async function exportVideo(chooseFolder = false) {
    if (!project || !edit) return
    if (!validTrim(edit.trimStartMs,edit.trimEndMs,project.durationMs)) { setError({ key: 'app.fixTrim' }); return }
    setBusy('exporting'); clearError(); setExported(null); setExportStage('app.savingEdit')
    try {
      await api.save(project.id,edit)
      const directory = chooseFolder ? await open({ directory: true, title: t('app.chooseExportFolderTitle') }) : null
      if (chooseFolder && !directory) { setBusy(null); return }
      const result = await api.export(project.id, typeof directory === 'string' ? directory : null, locale)
      setExported(result); setNotice('app.exportReady'); await refresh()
    } catch (reason) { fail(reason) } finally { setBusy(null); setModelPercent(null) }
  }
  const update = (patch: Partial<EditDocument>) => setEdit(current => current ? { ...current, ...patch } : current)
  const updateCrop = (key: keyof Rect, value: number) => setEdit(current => current ? { ...current, crop: { ...current.crop, [key]: value } } : current)
  const addCallout = () => {
    if (!edit) return
    const startMs = edit.trimStartMs
    const next: Callout = { startMs, endMs: Math.min(startMs+2500,edit.trimEndMs), text: t('app.newCallout'), x: .5, y: .82 }
    update({ callouts: [...edit.callouts,next] })
  }
  const updateCallout = (index: number, patch: Partial<Callout>) => update({ callouts: edit!.callouts.map((item,i) => i === index ? { ...item,...patch } : item) })

  return <div className="app-shell">
    <aside className="sidebar">
      <div className="brand"><span className="brand-mark"><Clapperboard size={19} strokeWidth={2.3} /></span><span>screenbox</span></div>
      <button className={`nav-item ${!project ? 'active' : ''}`} onClick={() => { setSelectedProject(null); setEdit(null); clearError(); setNotice('') }}><Circle size={16} /> {t('app.newRecording')}</button>
      <div className="sidebar-heading"><span>{t('app.library')}</span><span>{projects.length}</span></div>
      <div className="project-list">{projects.length === 0 ? <p className="sidebar-empty">{t('app.noRecordings')}</p> : projects.map(item => <button key={item.id} className={`project-item ${item.id === selectedProject ? 'active' : ''}`} onClick={() => setSelectedProject(item.id)}>
        <span className="project-icon"><Video size={17} /></span><span className="project-copy"><strong>{item.title}</strong><small>{dateLabel(item.createdAt, locale)} · {clock(item.durationMs)}</small></span>
      </button>)}</div>
      <div className="sidebar-footer"><span className="storage-dot" /> {t('app.localOnly')} <small>{t('app.localFiles')}</small></div>
    </aside>
    <main className="workspace">
      <header className="topbar"><div className="breadcrumbs">{t('app.workspace')} <span>/</span> <strong>{project ? t('app.editRecording') : t('app.newRecording')}</strong></div><div className="topbar-actions"><div className="top-status"><span className="status-dot" /> {t('app.privateWorkspace')}</div><SettingsMenu /></div></header>
      <div className="workspace-content">
        {currentError && <div className="message error" role="alert"><span>{currentError}</span><button onClick={() => { clearError(); if (settingsError) void updatePreferences(preferences); else void refresh() }}>{t('app.retry')}</button></div>}
        {notice && <div className="message success" role="status"><Check size={17} /><span>{t(notice)}</span><button aria-label={t('app.dismiss')} onClick={() => setNotice('')}><X size={15} /></button></div>}
        {busy === 'loading' ? <div className="loading-state"><LoaderCircle className="spin" size={28} /> {t('app.loading')}</div> : project && edit ? <>
          <div className="page-intro editor-intro"><button className="back-link" onClick={() => setSelectedProject(null)}><ArrowLeft size={16} /> {t('app.allRecordings')}</button><div className="intro-row"><div><h1>{project.title}</h1><p>{t('app.editFreely')}</p></div><button className="primary-button" disabled={!!busy} onClick={() => void exportVideo()}>{busy === 'exporting' ? <LoaderCircle className="spin" size={18} /> : <Sparkles size={18} />} {t('app.exportVideo')}</button></div></div>
          <div className="editor-grid"><section className="preview-panel"><div className="preview-toolbar"><span><span className="preview-dot" /> {t('app.preview')}</span><span>{project.width} × {project.height} · {clock(project.durationMs)}</span></div><div className="video-stage"><div className="video-frame" style={{ padding: `${edit.padding*100}%` }}><div className="preview-viewport"><video src={convertFileSrc(project.originalPath)} poster={convertFileSrc(project.originalPath.replace(/\.[^.]+$/, ".poster.jpg"))} preload="metadata" ref={videoRef} playsInline onPlay={() => setPlaying(true)} onPause={() => setPlaying(false)} onLoadedMetadata={event => { event.currentTarget.currentTime = edit.trimStartMs/1000; setPreviewTime(edit.trimStartMs) }} onTimeUpdate={event => { const time = event.currentTarget.currentTime*1000; setPreviewTime(time); if (time >= edit.trimEndMs) { event.currentTarget.pause(); event.currentTarget.currentTime = edit.trimStartMs/1000 } }} style={{ transform: `scale(${edit.zoomStrength})`, objectPosition: `${(edit.crop.x+edit.crop.width/2)*100}% ${(edit.crop.y+edit.crop.height/2)*100}%` }} /></div></div><button className="preview-play" aria-label={playing ? t('app.pausePreview') : t('app.playPreview')} onClick={() => { const video = videoRef.current; if (!video) return; if (playing) video.pause(); else { if (video.currentTime*1000 < edit.trimStartMs || video.currentTime*1000 >= edit.trimEndMs) video.currentTime = edit.trimStartMs/1000; void video.play() } }}>{playing ? <Pause size={23} fill="currentColor" /> : <Play size={23} fill="currentColor" />}</button>{edit.cursorHighlight && cursorPoint && cursorPoint.x >= edit.crop.x && cursorPoint.x <= edit.crop.x + edit.crop.width && cursorPoint.y >= edit.crop.y && cursorPoint.y <= edit.crop.y + edit.crop.height && <span className="preview-cursor" style={{ left: `${clamp(.5 + ((cursorPoint.x - edit.crop.x)/edit.crop.width - .5)*edit.zoomStrength*(1-edit.padding*2),0,1)*100}%`, top: `${clamp(.5 + ((cursorPoint.y - edit.crop.y)/edit.crop.height - .5)*edit.zoomStrength*(1-edit.padding*2),0,1)*100}%` }}><MousePointer2 size={17} /></span>}{activeCallout && <span className="preview-callout" style={{ left: `${activeCallout.x*100}%`, top: `${activeCallout.y*100}%`, bottom: 'auto' }}>{activeCallout.text}</span>}</div><div className="preview-scrubber"><span>{clock(previewTime)}</span><input aria-label={t('app.previewPosition')} type="range" min={edit.trimStartMs} max={edit.trimEndMs} value={clamp(previewTime,edit.trimStartMs,edit.trimEndMs)} onChange={event => { const ms = Number(event.target.value); if (videoRef.current) videoRef.current.currentTime = ms/1000; setPreviewTime(ms) }} /><span>{clock(edit.trimEndMs)}</span></div><div className="timeline"><div className="timeline-label"><Scissors size={16} /> {t('app.trimRange')} <span>{clock(edit.trimEndMs-edit.trimStartMs)}</span></div><div className="trim-inputs"><label>{t('app.start')} <input type="number" min="0" max={project.durationMs/1000} step="0.1" value={edit.trimStartMs/1000} onChange={event => update({ trimStartMs: Math.round(Number(event.target.value)*1000) })} /> {t('app.seconds')}</label><div className="trim-track"><span style={{ left: `${edit.trimStartMs/project.durationMs*100}%`, right: `${(1-edit.trimEndMs/project.durationMs)*100}%` }} /></div><label>{t('app.end')} <input type="number" min="0" max={project.durationMs/1000} step="0.1" value={edit.trimEndMs/1000} onChange={event => update({ trimEndMs: Math.round(Number(event.target.value)*1000) })} /> {t('app.seconds')}</label></div></div></section>
          <section className="controls-panel"><div className="controls-heading"><Settings2 size={17} /><strong>{t('app.finishFrame')}</strong></div><div className="control-section"><div className="control-title"><WandSparkles size={17} /><span>{t('app.autoFraming')}</span></div><label className="switch-row">{t('app.followCursor')} <input type="checkbox" checked={edit.autoZoom} onChange={event => update({ autoZoom: event.target.checked })} /></label><label className="slider-row">{t('app.zoomStrength')} <strong>{edit.zoomStrength.toFixed(2)}×</strong><input type="range" min="1" max="2.5" step="0.05" value={edit.zoomStrength} onChange={event => update({ zoomStrength: Number(event.target.value) })} /></label><label className="slider-row">{t('app.padding')} <strong>{Math.round(edit.padding*100)}%</strong><input type="range" min="0" max="0.2" step="0.01" value={edit.padding} onChange={event => update({ padding: Number(event.target.value) })} /></label><label className="switch-row">{t('app.highlightCursor')} <input type="checkbox" checked={edit.cursorHighlight} onChange={event => update({ cursorHighlight: event.target.checked })} /></label></div>
          <div className="control-section"><div className="control-title"><Scissors size={17} /><span>{t('app.crop')}</span></div><div className="crop-grid">{(['x','y','width','height'] as const).map(key => <label key={key}>{key === 'x' ? t('app.left') : key === 'y' ? t('app.top') : key === 'width' ? t('app.width') : t('app.height')}<input type="number" min="0" max="100" step="1" value={Math.round(edit.crop[key]*100)} onChange={event => updateCrop(key,Number(event.target.value)/100)} /></label>)}</div><button className="text-button" onClick={() => update({ crop: fullRect })}>{t('app.resetCrop')}</button></div>
          <div className="control-section"><div className="control-title"><Plus size={17} /><span>{t('app.textCallouts')}</span></div>{edit.callouts.map((item,index) => <div className="callout-item" key={index}><div className="callout-line"><input aria-label={t('app.calloutText', { number: index+1 })} maxLength={200} value={item.text} onChange={event => updateCallout(index,{ text: event.target.value })} /><button aria-label={t('app.removeCallout', { number: index+1 })} onClick={() => update({ callouts: edit.callouts.filter((_,i) => i !== index) })}><X size={15} /></button></div><div className="callout-times"><label>{t('app.from')} <input type="number" min="0" step="0.1" value={item.startMs/1000} onChange={event => updateCallout(index,{ startMs: Math.round(Number(event.target.value)*1000) })} /></label><label>{t('app.to')} <input type="number" min="0" step="0.1" value={item.endMs/1000} onChange={event => updateCallout(index,{ endMs: Math.round(Number(event.target.value)*1000) })} /></label><label>{t('app.left')} <input type="number" min="0" max="100" step="1" value={Math.round(item.x*100)} onChange={event => updateCallout(index,{ x: Number(event.target.value)/100 })} /></label><label>{t('app.top')} <input type="number" min="0" max="100" step="1" value={Math.round(item.y*100)} onChange={event => updateCallout(index,{ y: Number(event.target.value)/100 })} /></label></div></div>)}<button className="add-button" onClick={addCallout}><Plus size={15} />{t('app.addCallout')}</button></div>
          <div className="control-section"><div className="control-title"><Video size={17} /><span>{t('app.outputFormat')}</span></div><div className="preset-row"><button className={edit.preset === 'h264' ? 'selected' : ''} onClick={() => update({ preset: 'h264' })}>H.264 <small>{t('app.easyToShare')}</small></button><button className={edit.preset === 'webm' ? 'selected' : ''} onClick={() => update({ preset: 'webm' })}>WebM <small>{t('app.smallerFile')}</small></button></div></div><div className="controls-actions"><button className="secondary-button" disabled={!!busy} onClick={() => void save()}>{busy === 'saving' ? t('app.saving') : t('app.saveEdit')}</button><button className="primary-button" disabled={!!busy} onClick={() => void exportVideo()}>{busy === 'exporting' ? t('app.exporting') : t('app.exportVideo')}</button></div><button className="choose-folder" disabled={!!busy} onClick={() => void exportVideo(true)}>{t('app.chooseExportFolder')}</button></section></div>
          {busy === 'exporting' && <div className="export-progress" role="status"><LoaderCircle className="spin" size={18} /><div><strong>{t(exportStage)}</strong>{modelPercent !== null && <small>{t('app.speechModelDownload', { percent: modelPercent })}</small>}</div></div>}
          {exported && <div className="export-result"><div><span className="result-icon"><Check size={20} /></span><h2>{t('app.videoReady')}</h2><p>{t('app.companionFiles')}</p>{exported.transcriptionWarning && <p className="warning-text">{t('app.transcriptNeedsAttention', { detail: exported.transcriptionWarning })}</p>}</div><div className="result-actions"><button className="secondary-button" onClick={() => void api.copy(exported.videoPath).then(() => setNotice('app.filePathCopied')).catch(fail)}><Copy size={16} />{t('app.copyFilePath')}</button><button className="secondary-button" onClick={() => void api.reveal(exported.videoPath).catch(fail)}><FolderOpen size={16} />{t('app.revealInFinder')}</button></div></div>}
        </> : <>
          <div className="page-intro"><span className="section-label">{t('app.createSomethingClear')}</span><h1>{t('app.recordHeadline')}<br /><em>{t('app.recordSubheadline')}</em></h1><p>{t('app.recordDescription')}</p></div>
          <div className="record-layout"><section className="record-card"><div className="card-top"><div className="card-icon"><Monitor size={23} /></div><div><h2>{t('app.chooseWhatToRecord')}</h2><p>{t('app.selectSource')}</p></div></div>{sources.length === 0 ? <div className="empty-sources"><Monitor size={27} /><strong>{t('app.noCaptureSources')}</strong><p>{t('app.allowScreenRecording')}</p><button className="secondary-button" onClick={() => void refresh().catch(fail)}>{t('app.refreshSources')}</button></div> : <><label className="field-label" htmlFor="source-select">{t('app.recordingSource')}</label><div className="select-wrap"><select id="source-select" value={selectedSource} onChange={event => setSelectedSource(event.target.value)}>{sources.map(source => <option key={`${source.kind}:${source.id}`} value={`${source.kind}:${source.id}`}>{sourceLabel(source.kind, locale)} · {source.title}</option>)}</select><ChevronDown size={17} /></div>{selected?.kind === 'region' && <RegionPicker value={region} onChange={setRegion} locale={locale} />}<div className="capture-options"><div className="option-icon"><Mic size={19} /></div><div><strong>{t('app.microphoneAudio')}</strong><small>{t('app.addYourVoice')}</small></div><input type="checkbox" aria-label={t('app.microphoneAudio')} checked={microphone} onChange={event => setMicrophone(event.target.checked)} /></div><div className="record-action">{recording ? <button className="stop-button" disabled={!!busy} onClick={() => void stop()}><Square size={17} fill="currentColor" />{t('app.stopRecording')} <span>{clock(elapsed)}</span></button> : <button className="record-button" disabled={!!busy} onClick={() => void start()}><span className="record-circle" /> {busy === 'recording' ? t('app.starting') : t('app.startRecording')}</button>}<p>{t('app.recordingsStayLocal')}</p></div></>}</section><aside className="workflow-note"><span className="note-graphic"><MousePointer2 size={36} /></span><h3>{t('app.cursorFocusHeadline')}</h3><p>{t('app.cursorFocusDescription')}</p><div className="note-steps"><span><strong>1</strong>{t('app.recordStep')}</span><span><strong>2</strong>{t('app.refineStep')}</span><span><strong>3</strong>{t('app.exportStep')}</span></div></aside></div>
          {projects.length === 0 && <div className="first-run-note"><span className="sparkle-icon"><Sparkles size={18} /></span><div><strong>{t('app.firstRecording')}</strong><p>{t('app.firstRecordingHint')}</p></div></div>}
        </>}
      </div>
    </main>
  </div>
}

export default function App() {
  const [preferences, setPreferences] = useState<UserPreferences>(defaultPreferences)
  const [preferencesReady, setPreferencesReady] = useState(false)
  const [settingsError, setSettingsError] = useState<AppError | null>(null)
  const [systemLanguage, setSystemLanguage] = useState(() => navigator.language)
  const [systemIsDark, setSystemIsDark] = useState(() => window.matchMedia('(prefers-color-scheme: dark)').matches)
  const locale = resolveLanguage(preferences.language, systemLanguage)
  const theme = resolveTheme(preferences.theme, systemIsDark)

  useEffect(() => {
    let mounted = true
    api.preferences().then(value => { if (mounted) setPreferences(value) })
      .catch(reason => { if (mounted) setSettingsError({ key: 'app.settingsLoadFailed', detail: reason }) })
      .finally(() => { if (mounted) setPreferencesReady(true) })
    return () => { mounted = false }
  }, [])
  useEffect(() => {
    let mounted = true
    let unlisten: (() => void) | undefined
    void listen<UserPreferences>('preferences-changed', event => setPreferences(event.payload)).then(stop => {
      if (mounted) unlisten = stop
      else stop()
    })
    return () => { mounted = false; unlisten?.() }
  }, [])
  useEffect(() => {
    const media = window.matchMedia('(prefers-color-scheme: dark)')
    const updateSystemTheme = () => setSystemIsDark(media.matches)
    const updateSystemLanguage = () => setSystemLanguage(navigator.language)
    media.addEventListener('change', updateSystemTheme)
    window.addEventListener('languagechange', updateSystemLanguage)
    return () => {
      media.removeEventListener('change', updateSystemTheme)
      window.removeEventListener('languagechange', updateSystemLanguage)
    }
  }, [])
  useEffect(() => {
    document.documentElement.dataset.theme = theme
    document.documentElement.style.colorScheme = theme
  }, [theme])
  useEffect(() => {
    if (preferencesReady) void api.setTrayLanguage(locale).catch(() => {})
  }, [locale, preferencesReady])

  const updatePreferences = async (next: UserPreferences) => {
    const previous = preferences
    setPreferences(next)
    try {
      await api.savePreferences(next)
      setSettingsError(null)
    } catch (reason) {
      setPreferences(previous)
      setSettingsError({ key: 'app.settingsUpdateFailed', detail: reason })
    }
  }
  const context: UiContextValue = { locale, theme, preferences, preferencesReady, settingsError, updatePreferences }
  const initialLocale = resolveLanguage('system', systemLanguage)
  return <UiContext.Provider value={context}>
    {isOverlay ? <TimerOverlay /> : preferencesReady ? <AppMain /> : <div className="loading-state">{translate(initialLocale, 'app.loading')}</div>}
  </UiContext.Provider>
}
