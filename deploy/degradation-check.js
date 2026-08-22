// PRD A7 的可复现服务故障检查。脚本只停止推理容器，并在退出时自动恢复。
const { spawnSync } = require('node:child_process')
const path = require('node:path')

const BASE = process.argv[2] || 'http://127.0.0.1:18080'
const USERNAME = process.env.AUTH_USERNAME
const PASSWORD = process.env.AUTH_PASSWORD
const COMPOSE_FILE = path.join(__dirname, 'docker-compose.yml')
let token = ''
const stopped = new Set()
let shuttingDown = false

function assert(condition, message) {
  if (!condition) throw new Error(message)
}

function compose(args, capture = false) {
  const result = spawnSync('docker', ['compose', '-f', COMPOSE_FILE, ...args], {
    cwd: __dirname,
    encoding: 'utf8',
    stdio: capture ? 'pipe' : 'inherit',
  })
  if (result.status !== 0) {
    throw new Error(`docker compose ${args.join(' ')} 失败${capture ? `：${result.stderr.trim()}` : ''}`)
  }
  return (result.stdout || '').trim()
}

function requireRunning(service) {
  assert(compose(['ps', '-q', '--status', 'running', service], true), `${service} 当前未运行，未执行故障注入`)
}

function stop(service) {
  compose(['stop', service])
  stopped.add(service)
}

function restore(service) {
  if (!stopped.has(service)) return
  compose(['start', service])
  stopped.delete(service)
}

async function api(pathname, options = {}) {
  const response = await fetch(BASE + pathname, {
    ...options,
    headers: {
      ...(options.body instanceof FormData ? {} : { 'Content-Type': 'application/json' }),
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
      ...(options.headers || {}),
    },
  })
  const body = await response.json()
  if (!response.ok) throw new Error(`${pathname}: ${response.status} ${JSON.stringify(body)}`)
  return body
}

function serviceHealth(service) {
  const id = compose(['ps', '-q', '--status', 'running', service], true)
  if (!id) return false
  const result = spawnSync('docker', ['inspect', '--format', '{{.State.Health.Status}}', id], {
    encoding: 'utf8',
  })
  return result.status === 0 && result.stdout.trim() === 'healthy'
}

async function waitFor(label, check, timeoutMs = 60000) {
  const deadline = Date.now() + timeoutMs
  let lastError
  while (Date.now() < deadline) {
    try {
      if (await check()) return
    } catch (error) {
      lastError = error
    }
    await new Promise((resolve) => setTimeout(resolve, 1000))
  }
  throw new Error(`${label} 超时${lastError ? `：${lastError.message}` : ''}`)
}

function silentWav() {
  const sampleRate = 16000
  const samples = sampleRate
  const buffer = Buffer.alloc(44 + samples * 2)
  buffer.write('RIFF', 0)
  buffer.writeUInt32LE(buffer.length - 8, 4)
  buffer.write('WAVEfmt ', 8)
  buffer.writeUInt32LE(16, 16)
  buffer.writeUInt16LE(1, 20)
  buffer.writeUInt16LE(1, 22)
  buffer.writeUInt32LE(sampleRate, 24)
  buffer.writeUInt32LE(sampleRate * 2, 28)
  buffer.writeUInt16LE(2, 32)
  buffer.writeUInt16LE(16, 34)
  buffer.write('data', 36)
  buffer.writeUInt32LE(samples * 2, 40)
  return buffer
}

async function login() {
  assert(USERNAME && PASSWORD, '请通过 AUTH_USERNAME 和 AUTH_PASSWORD 提供验收账号')
  const result = await api('/api/auth/login', {
    method: 'POST',
    body: JSON.stringify({ username: USERNAME, password: PASSWORD }),
  })
  token = result.token
}

async function textFallback() {
  const result = await api('/api/ask/text', {
    method: 'POST',
    body: JSON.stringify({ text: '杯子' }),
  })
  assert(['hit', 'tts_only_down'].includes(result.status), `文字提问不可用：${result.status}`)
  assert(result.result?.target_id === 'word_cup', '文字提问没有返回 cup')
  return result.status
}

async function ttsFallbackOutcome() {
  const text = `A7 fixed service fallback check ${crypto.randomUUID()}`
  const response = await fetch(`${BASE}/api/tts/audio?text=${encodeURIComponent(text)}&voice=en_US-mike-medium&rate=0.8`, {
    headers: { Authorization: `Bearer ${token}` },
  })
  if (response.ok) return { status: response.status, outcome: 'remote_audio_available' }
  const body = await response.json()
  assert(response.status === 503 && body.code === 'tts_unavailable', `TTS 故障结果异常：${response.status} ${JSON.stringify(body)}`)
  return { status: response.status, outcome: 'audio_unavailable_text_retained' }
}

async function main() {
  requireRunning('backend')
  requireRunning('tts')
  requireRunning('asr')
  await login()

  stop('tts')
  const ttsFallback = await ttsFallbackOutcome()
  const ttsTextStatus = await textFallback()
  restore('tts')
  await waitFor('TTS 容器恢复健康', async () => serviceHealth('tts'), 120000)

  stop('asr')
  await waitFor('后端识别 ASR 已停止', async () => (await api('/api/readyz')).services.asr === false, 45000)
  const form = new FormData()
  form.append('audio', new Blob([silentWav()], { type: 'audio/wav' }), 'a7-silence.wav')
  const voiceResult = await api('/api/ask/voice', { method: 'POST', body: form })
  assert(voiceResult.status === 'asr_fail', `ASR 停止后应返回 asr_fail，实际为 ${voiceResult.status}`)
  const asrTextStatus = await textFallback()
  restore('asr')
  await waitFor('ASR 恢复就绪', async () => (await api('/api/readyz')).services.asr === true, 120000)

  console.log(JSON.stringify({
    a7_tts_container_stopped: true,
    a7_tts_audio_http_status: ttsFallback.status,
    a7_tts_outcome: ttsFallback.outcome,
    a7_tts_text_path_status: ttsTextStatus,
    a7_asr_container_stopped: true,
    a7_asr_voice_status: voiceResult.status,
    a7_asr_text_path_status: asrTextStatus,
    services_restored: true,
    manual_ui_evidence: false,
    note: '自动检查不替代移动端对降级提示、打字兜底和恢复体验的人工证据。',
  }, null, 2))
}

async function shutdown(error) {
  if (shuttingDown) return
  shuttingDown = true
  try { restore('tts') } catch (restoreError) { console.error(`恢复 tts 失败：${restoreError.message}`) }
  try { restore('asr') } catch (restoreError) { console.error(`恢复 asr 失败：${restoreError.message}`) }
  if (error) {
    console.error(error.message)
    process.exitCode = 1
  }
}

process.once('SIGINT', () => shutdown(new Error('收到 SIGINT，已恢复服务')).then(() => process.exit(130)))
process.once('SIGTERM', () => shutdown(new Error('收到 SIGTERM，已恢复服务')).then(() => process.exit(143)))
main().then(() => shutdown()).catch(shutdown)
