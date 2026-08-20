// PRD A1～A4 的可自动化 API 验收。真机语音性能、人工试听与真机闭环另行记录。
const words = require('../data/seed/words.json')

const BASE = process.argv[2] || 'http://127.0.0.1:8080'
const USERNAME = process.env.AUTH_USERNAME
const PASSWORD = process.env.AUTH_PASSWORD
let token = ''

async function api(path, options = {}) {
  const response = await fetch(BASE + path, {
    headers: { 'Content-Type': 'application/json', ...(token ? { Authorization: `Bearer ${token}` } : {}), ...(options.headers || {}) },
    ...options,
  })
  const body = await response.json()
  if (!response.ok) throw new Error(`${path}: ${response.status} ${JSON.stringify(body)}`)
  return body
}

async function ask(text) {
  return api('/api/ask/text', { method: 'POST', body: JSON.stringify({ text }) })
}

function assert(condition, message) {
  if (!condition) throw new Error(message)
}

async function main() {
  assert(USERNAME && PASSWORD, '请通过 AUTH_USERNAME 和 AUTH_PASSWORD 提供验收账号')
  const login = await api('/api/auth/login', {
    method: 'POST',
    body: JSON.stringify({ username: USERNAME, password: PASSWORD }),
  })
  token = login.token
  const me = await api('/api/family/me')
  if (!me.initialized) {
    await api('/api/family/init', {
      method: 'POST',
      body: JSON.stringify({ child_name: 'release-check', child_birthdate: '2024-10-01' }),
    })
  }

  const timings = []
  for (let i = 0; i < 20; i += 1) {
    const started = performance.now()
    const result = await ask('杯子')
    timings.push(performance.now() - started)
    assert(result.result?.target_id === 'word_cup', 'A1：库内词「杯子」未命中 cup')
  }
  timings.sort((a, b) => a - b)
  const p95 = timings[Math.ceil(timings.length * 0.95) - 1]

  let aliasCount = 0
  for (const word of words) {
    assert(word.aliases.length >= 3, `A2：${word.id} 少于 3 个口语别名`)
    for (const alias of word.aliases.slice(0, 3)) {
      const result = await ask(alias)
      assert(result.result?.target_id === word.id, `A2：${word.id} 的别名「${alias}」命中 ${result.result?.target_id || result.status}`)
      aliasCount += 1
    }
  }

  const homophones = [
    ['被子', 'word_cup'],
    ['晚', 'word_bowl'],
    ['死', 'word_four'],
    ['求', 'word_ball'],
    ['扯', 'word_car'],
    ['登', 'word_light'],
    ['闷', 'word_door'],
    ['猫子', 'word_hat'],
    ['苦子', 'word_pants'],
    ['密饭', 'word_rice'],
  ]
  for (const [text, expected] of homophones) {
    const result = await ask(text)
    const candidates = result.candidates?.map((item) => item.target_id) || []
    assert(result.result?.target_id === expected || candidates.includes(expected), `A3：「${text}」没有纠正或候选到 ${expected}`)
  }

  const misses = ['口红', '雨伞', '牙刷', '毛巾', '冰箱', '电视', '钥匙', '书包', '蜡笔', '滑梯']
  for (const text of misses) {
    const result = await ask(`妈妈的${text}`)
    assert(result.status === 'nomatch', `A4：库外词「${text}」意外返回 ${result.status}`)
    assert(result.unmatched_id, `A4：库外词「${text}」没有写入 unmatched_query`)
    assert(result.candidates?.length > 0, `A4：库外词「${text}」没有相近词推荐`)
  }
  const unmatched = await api('/api/unmatched?limit=100')
  for (const text of misses) {
    assert(unmatched.unmatched.some((item) => item.normalized_text === text), `A4：未命中表缺少「${text}」`)
  }

  console.log(JSON.stringify({
    a1_text_api_p95_ms: Number(p95.toFixed(1)),
    a1_note: '仅本机文字 API 基线，不替代局域网中端安卓语音 P95 验收',
    a2_aliases_passed: aliasCount,
    a3_homophones_passed: homophones.length,
    a4_misses_passed: misses.length,
  }, null, 2))
}

main().catch((error) => {
  console.error(error.message)
  process.exit(1)
})
