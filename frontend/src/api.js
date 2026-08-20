// API 客户端（REST + 流式，PRD 9.3）
// 统一错误处理：后端返回 { ok, code, message }，前端按 code 分支降级（5.4 / 9.10）

const BASE = '/api'
const TOKEN_KEY = 'babyeng_auth_token'

export const authToken = () => localStorage.getItem(TOKEN_KEY) || ''

async function request(path, options = {}) {
  let res
  try {
    res = await fetch(BASE + path, {
      headers: {
        'Content-Type': 'application/json',
        ...(authToken() ? { Authorization: `Bearer ${authToken()}` } : {}),
        ...(options.headers || {}),
      },
      ...options,
    })
  } catch (e) {
    throw new ApiError('network', '连不上服务器', e)
  }
  if (!res.ok) {
    let body = null
    try { body = await res.json() } catch { /* ignore */ }
    if (res.status === 401) window.dispatchEvent(new Event('babyeng:unauthorized'))
    throw new ApiError(body?.code || 'http_' + res.status, body?.message || `请求失败(${res.status})`, null, res.status)
  }
  const ct = res.headers.get('content-type') || ''
  if (ct.includes('application/json')) return res.json()
  return res
}

export class ApiError extends Error {
  constructor(code, message, cause, status) {
    super(message)
    this.code = code
    this.cause = cause
    this.status = status
  }
  /** 是否属「服务不可用」类降级（TTS/ASR/LLM 挂掉，前端给降级提示而不是白屏） */
  get degradable() {
    return ['tts_unavailable', 'asr_unavailable', 'llm_unavailable', 'network'].includes(this.code)
  }
}

export const api = {
  // ---------- 账号 ----------
  login: async (username, password) => {
    const result = await request('/auth/login', { method: 'POST', body: JSON.stringify({ username, password }) })
    localStorage.setItem(TOKEN_KEY, result.token)
    return result
  },
  authMe: () => request('/auth/me'),
  logout: async () => {
    try { await request('/auth/logout', { method: 'POST' }) } finally { localStorage.removeItem(TOKEN_KEY) }
  },
  // ---------- 家庭 / 引导 ----------
  familyMe: () => request('/family/me'),
  familyInit: (data) => request('/family/init', { method: 'POST', body: JSON.stringify(data) }),
  familySettings: (settings) => request('/family/settings', { method: 'PUT', body: JSON.stringify({ settings }) }),
  childUpdate: (childId, data) => request(`/family/child/${childId}`, { method: 'PUT', body: JSON.stringify(data) }),

  // ---------- 问答（M1） ----------
  askText: (text, opts = {}) =>
    request('/ask/text', { method: 'POST', body: JSON.stringify({ text, ...opts }) }),
  askVoice: (blob, fileName, childId, familyId) => {
    const form = new FormData()
    form.append('audio', blob, fileName)
    if (childId) form.append('child_id', childId)
    if (familyId) form.append('family_id', familyId)
    return authFetch(BASE + '/ask/voice', { method: 'POST', body: form }).then(handleJson)
  },
  askConfirm: (targetType, targetId, childId) =>
    request('/ask/confirm', { method: 'POST', body: JSON.stringify({ target_type: targetType, target_id: targetId, child_id: childId }) }),

  // ---------- TTS ----------
  ttsUrl: (text, rate = 0.8, voice = 'en_US-lessac-medium') =>
    `${BASE}/tts/audio?text=${encodeURIComponent(text)}&voice=${encodeURIComponent(voice)}&rate=${rate}&access_token=${encodeURIComponent(authToken())}`,

  // ---------- 词库 / 场景（M5 / M2） ----------
  words: (params = '') => request(`/words${params}`),
  wordDetail: (id) => request(`/words/${id}`),
  sentences: (params = '') => request(`/sentences${params}`),
  scenes: (childId = '') => request(`/scenes${childId ? `?child_id=${childId}` : ''}`),

  // ---------- 录音（M3） ----------
  uploadRecording: (blob, fileName, { childId, targetType, targetId, durationMs }) => {
    const form = new FormData()
    form.append('audio', blob, fileName)
    form.append('child_id', childId)
    form.append('target_type', targetType)
    form.append('target_id', targetId)
    form.append('duration_ms', String(durationMs))
    return authFetch(BASE + '/recordings', { method: 'POST', body: form }).then(handleJson)
  },
  recordings: (childId) => request(`/recordings?child_id=${childId}`),
  recordingUrl: (id) => `${BASE}/recordings/${id}/audio?access_token=${encodeURIComponent(authToken())}`,
  favoriteRecording: (id, favorited) => request(`/recordings/${id}/favorite?favorited=${favorited}`, { method: 'POST' }),
  deleteRecording: (id) => request(`/recordings/${id}`, { method: 'DELETE' }),
  cleanupExpired: () => request('/recordings/cleanup-expired', { method: 'POST' }),

  // ---------- 数据隐私（11.4） ----------
  exportData: () => request('/data/export'),
  clearData: () => request('/data/clear', {
    method: 'POST',
    body: JSON.stringify({ confirmation: 'DELETE_ALL_LEARNING_DATA' }),
  }),

  // ---------- 学习记录 / 进度 / 复习（M2 / 8.6） ----------
  recordLearning: (data) => request('/learning-records', { method: 'POST', body: JSON.stringify(data) }),
  progressSummary: (childId) => request(`/progress/summary?child_id=${childId}`),
  reviewQueue: (childId) => request(`/review/queue?child_id=${childId}`),
  wordProgress: (childId, targetId) => request(`/progress/word?child_id=${childId}&target_id=${targetId}`),

  // ---------- 日报 / 成就（M8 / M6） ----------
  reportToday: (childId) => request(`/report/today?child_id=${childId}`),
  reportCalendar: (childId) => request(`/report/calendar?child_id=${childId}`),
  achievements: (childId) => request(`/achievements?child_id=${childId}`),
  recordingsToday: (childId) => request(`/report/recordings-today?child_id=${childId}`),

  // ---------- 未命中表（8.8） ----------
  unmatched: () => request('/unmatched'),
}

async function handleJson(res) {
  if (!res.ok) {
    let body = null
    try { body = await res.json() } catch { /* ignore */ }
    throw new ApiError(body?.code || 'http_' + res.status, body?.message || `请求失败(${res.status})`)
  }
  return res.json()
}

function authFetch(url, options = {}) {
  return fetch(url, {
    ...options,
    headers: {
      ...(authToken() ? { Authorization: `Bearer ${authToken()}` } : {}),
      ...(options.headers || {}),
    },
  })
}
