import { ref } from 'vue'

// 幼儿侧录音（PRD 4.3 / 6.2）：
// 「点按开始 + 静音 1.5s VAD 自动停」，而不是「按住说话」（持续按压 2.5 岁前不稳定）
// - 时长 < 0.5s 不入库（5.4）
// - 15s 超时自动截断
// - iOS Safari MediaRecorder 输出 mp4/aac，Android/桌面输出 webm/opus（9.2 后端按 MIME 分支转码）

const SILENCE_MS = 1500 // 静音自动停阈值
const MAX_MS = 15000 // 超时截断

export function useRecorder() {
  const state = ref('idle') // idle | recording
  const durationMs = ref(0)
  const errorMsg = ref('')
  const isAvailable = ref(typeof navigator !== 'undefined' && !!navigator.mediaDevices?.getUserMedia && typeof MediaRecorder !== 'undefined')

  let stream = null
  let mediaRecorder = null
  let analyser = null
  let audioContext = null
  let chunks = []
  let startTs = 0
  let timer = null
  let silenceTimer = null
  let silenceStart = 0
  let stopPromise = null
  let resolveStop = null

  function pickMimeType() {
    const candidates = ['audio/webm;codecs=opus', 'audio/webm', 'audio/mp4', 'audio/ogg;codecs=opus']
    for (const m of candidates) {
      if (window.MediaRecorder.isTypeSupported(m)) return m
    }
    return ''
  }

  function extFromMime(mime) {
    if (!mime) return 'webm'
    if (mime.includes('mp4') || mime.includes('aac')) return 'm4a'
    if (mime.includes('ogg')) return 'ogg'
    return 'webm'
  }

  /** 点按开始（A 段由母亲代点，B 段幼儿自己点，PRD 6.2） */
  async function start() {
    if (state.value === 'recording') return
    errorMsg.value = ''
    try {
      // 请求权限（首次会弹窗；被拒由调用方 catch 处理）
      stream = await navigator.mediaDevices.getUserMedia({
        audio: {
          echoCancellation: true,
          noiseSuppression: true,
        },
      })
      audioContext = new (window.AudioContext || window.webkitAudioContext)()
      const source = audioContext.createMediaStreamSource(stream)
      analyser = audioContext.createAnalyser()
      analyser.fftSize = 512
      source.connect(analyser)

      const mime = pickMimeType()
      mediaRecorder = new MediaRecorder(stream, mime ? { mimeType: mime } : undefined)
      chunks = []
      stopPromise = new Promise((resolve) => { resolveStop = resolve })
      mediaRecorder.ondataavailable = (e) => {
        if (e.data && e.data.size > 0) chunks.push(e.data)
      }
      mediaRecorder.onstop = () => {
        cleanupStream()
        const elapsed = Date.now() - startTs
        durationMs.value = Math.min(elapsed, MAX_MS)
        if (elapsed < 500 || chunks.length === 0) {
          chunks = []
          resolveStop?.(null)
          resolveStop = null
          return
        }
        const recordedMime = mediaRecorder.mimeType || 'audio/webm'
        const blob = new Blob(chunks, { type: recordedMime })
        chunks = []
        resolveStop?.({
          blob,
          durationMs: Math.min(elapsed, MAX_MS),
          ext: extFromMime(recordedMime),
        })
        resolveStop = null
      }
      mediaRecorder.start(100)
      state.value = 'recording'
      startTs = Date.now()
      durationMs.value = 0

      // 计时 + VAD 静音检测
      timer = setInterval(() => {
        durationMs.value = Date.now() - startTs
        // 15s 超时自动截断（幼儿跟读一个词或一句话不会更长，6.2）
        if (durationMs.value >= MAX_MS) {
          stop()
          return
        }
        // 静音检测
        if (analyser) {
          const data = new Uint8Array(analyser.fftSize)
          analyser.getByteTimeDomainData(data)
          let sum = 0
          for (let i = 0; i < data.length; i++) {
            const v = (data[i] - 128) / 128
            sum += v * v
          }
          const rms = Math.sqrt(sum / data.length)
          if (rms < 0.02) {
            // 静音中
            if (!silenceStart) silenceStart = Date.now()
            else if (Date.now() - silenceStart >= SILENCE_MS) stop()
          } else {
            silenceStart = 0
          }
        }
      }, 100)
    } catch (e) {
      errorMsg.value = 'mic_permission_denied'
      cleanupStream()
      throw e
    }
  }

  /** 再点一次手动结束 / VAD 自动停 / 超时截断 */
  function stop() {
    if (state.value !== 'recording') return
    if (timer) clearInterval(timer)
    timer = null
    try {
      mediaRecorder?.stop()
    } catch {
      /* ignore */
    }
    state.value = 'idle'
  }

  /** 结束后的回调：返回 (blob, durationMs, ext)；过短时返回 null */
  function onStop() {
    if (!mediaRecorder || !stopPromise) return Promise.resolve(null)
    const completion = stopPromise
    stop()
    return completion
  }

  function cleanupStream() {
    if (timer) { clearInterval(timer); timer = null }
    if (stream) {
      stream.getTracks().forEach((t) => t.stop())
      stream = null
    }
    if (audioContext && audioContext.state !== 'closed') {
      audioContext.close().catch(() => {})
      audioContext = null
    }
    analyser = null
    silenceStart = 0
  }

  return { state, durationMs, errorMsg, isAvailable, start, stop, onStop }
}
