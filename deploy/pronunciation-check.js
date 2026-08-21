// PRD A6 自动预检：验证 58 条音频可读取并生成待人工试听、音标核对的证据清单。
const crypto = require('node:crypto')
const fs = require('node:fs')
const path = require('node:path')
const words = require('../data/seed/words.json')
const sentences = require('../data/seed/sentences.json')

const BASE = process.argv[2] || 'http://127.0.0.1:18080'
const OUTPUT = process.argv[3] || path.join('/tmp', `babyeng-a6-pronunciation-${new Date().toISOString().slice(0, 10)}.json`)
const USERNAME = process.env.AUTH_USERNAME
const PASSWORD = process.env.AUTH_PASSWORD
const VOICE = process.env.A6_TTS_VOICE || 'en_US-mike-medium'
const RATE = Number(process.env.A6_TTS_RATE || '0.8')
let token = ''

function assert(condition, message) {
  if (!condition) throw new Error(message)
}

async function jsonApi(pathname, options = {}) {
  const response = await fetch(BASE + pathname, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
      ...(options.headers || {}),
    },
  })
  const body = await response.json()
  if (!response.ok) throw new Error(`${pathname}: ${response.status} ${JSON.stringify(body)}`)
  return body
}

async function login() {
  assert(USERNAME && PASSWORD, '请通过 AUTH_USERNAME 和 AUTH_PASSWORD 提供验收账号')
  const result = await jsonApi('/api/auth/login', {
    method: 'POST',
    body: JSON.stringify({ username: USERNAME, password: PASSWORD }),
  })
  token = result.token
}

function auditItems() {
  return [
    ...words.filter((item) => item.review_status === 'published').map((item) => ({ ...item, type: 'word' })),
    ...sentences.filter((item) => item.review_status === 'published').map((item) => ({ ...item, type: 'sentence' })),
  ]
}

async function checkAudio(item, index, total) {
  const query = new URLSearchParams({ text: item.en, voice: VOICE, rate: String(RATE) })
  const response = await fetch(`${BASE}/api/tts/audio?${query}`, {
    headers: { Authorization: `Bearer ${token}` },
  })
  const contentType = response.headers.get('content-type') || ''
  const bytes = Buffer.from(await response.arrayBuffer())
  const audioOk = response.ok && contentType.startsWith('audio/') && bytes.length >= 100
  process.stderr.write(`[${index + 1}/${total}] ${item.id}: ${response.status}, ${bytes.length} bytes\n`)
  return {
    id: item.id,
    type: item.type,
    zh: item.zh,
    en: item.en,
    phonetic: item.phonetic,
    phonetic_source: item.phonetic_source,
    automatic: {
      audio_ok: audioOk,
      audio_http_status: response.status,
      content_type: contentType,
      byte_length: bytes.length,
      sha256: crypto.createHash('sha256').update(bytes).digest('hex'),
      phonetic_fields_present: Boolean(item.phonetic && item.phonetic_source),
    },
    manual: {
      listened: null,
      pronunciation_correct: null,
      natural_and_clear: null,
      phonetic_verified: null,
      reviewer: null,
      reviewed_at: null,
      notes: '',
    },
  }
}

async function main() {
  const items = auditItems()
  assert(items.length === 58, `A6 应有 58 条 published 内容，实际 ${items.length} 条`)
  assert(new Set(items.map((item) => item.id)).size === items.length, 'A6 内容 ID 不唯一')
  assert(items.every((item) => item.phonetic && item.phonetic_source), 'A6 存在缺少音标或来源的内容')
  assert(Number.isFinite(RATE) && RATE >= 0.5 && RATE <= 1.5, 'A6_TTS_RATE 必须在 0.5～1.5')
  await login()

  const checks = []
  for (let index = 0; index < items.length; index += 1) {
    try {
      checks.push(await checkAudio(items[index], index, items.length))
    } catch (error) {
      checks.push({
        id: items[index].id,
        type: items[index].type,
        zh: items[index].zh,
        en: items[index].en,
        phonetic: items[index].phonetic,
        phonetic_source: items[index].phonetic_source,
        automatic: { audio_ok: false, error: error.message, phonetic_fields_present: true },
        manual: { listened: null, pronunciation_correct: null, natural_and_clear: null, phonetic_verified: null, reviewer: null, reviewed_at: null, notes: '' },
      })
    }
  }

  const audioPassed = checks.filter((item) => item.automatic.audio_ok).length
  const evidence = {
    format: 'babyeng-a6-pronunciation-audit-v1',
    generated_at: new Date().toISOString(),
    base_url: BASE,
    voice: VOICE,
    rate: RATE,
    expected_count: 58,
    automatic_audio_passed: audioPassed,
    automatic_phonetic_fields_passed: checks.filter((item) => item.automatic.phonetic_fields_present).length,
    manual_complete: false,
    manual_completion_rule: '58 条 listened、pronunciation_correct、natural_and_clear、phonetic_verified 均为 true，且填写 reviewer 与 reviewed_at。',
    items: checks,
  }
  fs.mkdirSync(path.dirname(path.resolve(OUTPUT)), { recursive: true })
  fs.writeFileSync(OUTPUT, JSON.stringify(evidence, null, 2) + '\n', { mode: 0o600 })
  fs.chmodSync(OUTPUT, 0o600)
  console.log(JSON.stringify({ output: path.resolve(OUTPUT), automatic_audio_passed: audioPassed, expected_count: 58, manual_complete: false }, null, 2))
  if (audioPassed !== 58) process.exitCode = 1
}

function verifyManual(file) {
  assert(file, '请提供 A6 证据 JSON 路径')
  const evidence = JSON.parse(fs.readFileSync(file, 'utf8'))
  assert(evidence.format === 'babyeng-a6-pronunciation-audit-v1', 'A6 证据格式不支持')
  assert(Array.isArray(evidence.items) && evidence.items.length === 58, 'A6 证据必须恰好包含 58 条')
  const incomplete = evidence.items.filter((item) => {
    const manual = item.manual || {}
    return item.automatic?.audio_ok !== true ||
      manual.listened !== true ||
      manual.pronunciation_correct !== true ||
      manual.natural_and_clear !== true ||
      manual.phonetic_verified !== true ||
      !manual.reviewer ||
      !manual.reviewed_at
  })
  assert(incomplete.length === 0, `A6 尚有 ${incomplete.length} 条未完成：${incomplete.slice(0, 10).map((item) => item.id).join(', ')}`)
  console.log(JSON.stringify({ file: path.resolve(file), verified_count: 58, manual_complete: true }, null, 2))
}

if (process.argv[2] === '--verify-manual') {
  try {
    verifyManual(process.argv[3])
  } catch (error) {
    console.error(error.message)
    process.exit(1)
  }
} else {
  main().catch((error) => {
    console.error(error.message)
    process.exit(1)
  })
}
