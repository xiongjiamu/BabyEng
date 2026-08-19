import { shallowRef } from 'vue'

// 音频播放（PRD 4.1 / 6.4）：
// - 首次播放必须由用户手势触发以解锁 AudioContext（iOS 限制，PRD 9.2）
// - 慢速播放支持（默认 0.8x，幼儿捕捉音节用，PRD 4.4）
export function useAudio() {
  const ctxRef = shallowRef(null)

  function unlock() {
    // 在录音/播放按钮的点击回调里调用（PRD 4.1 第 7 步：解锁后本次会话可自动播放）
    if (!ctxRef.value) {
      try {
        ctxRef.value = new (window.AudioContext || window.webkitAudioContext)()
      } catch {
        return null
      }
    }
    if (ctxRef.value.state === 'suspended') ctxRef.value.resume()
    return ctxRef.value
  }

  async function playUrl(url, { rate = 1 } = {}) {
    const ctx = unlock()
    if (!ctx) return
    try {
      const resp = await fetch(url)
      if (!resp.ok) throw new Error('audio fetch failed: ' + resp.status)
      const arrayBuf = await resp.arrayBuffer()
      const audioBuf = await ctx.decodeAudioData(arrayBuf)
      const src = ctx.createBufferSource()
      src.buffer = audioBuf
      src.playbackRate.value = rate
      const gain = ctx.createGain()
      gain.gain.value = 1
      src.connect(gain)
      gain.connect(ctx.destination)
      return new Promise((resolve) => {
        src.onended = () => resolve(true)
        src.start()
      })
    } catch (e) {
      console.warn('播放失败', e)
      return false
    }
  }

  /** 播放本地 blob（录音回放） */
  async function playBlob(blob, { rate = 1 } = {}) {
    const ctx = unlock()
    if (!ctx) return
    try {
      const url = URL.createObjectURL(blob)
      const resp = await fetch(url)
      const arrayBuf = await resp.arrayBuffer()
      const audioBuf = await ctx.decodeAudioData(arrayBuf)
      const src = ctx.createBufferSource()
      src.buffer = audioBuf
      src.playbackRate.value = rate
      src.connect(ctx.destination)
      return new Promise((resolve) => {
        src.onended = () => {
          URL.revokeObjectURL(url)
          resolve(true)
        }
        src.start()
      })
    } catch (e) {
      console.warn('blob 播放失败', e)
      return false
    }
  }

  return { unlock, playUrl, playBlob }
}
