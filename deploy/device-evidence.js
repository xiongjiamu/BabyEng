// PRD A1/A5/A7/A8 与屏幕计时真机证据模板及严格校验器。
const fs = require('node:fs')
const path = require('node:path')

const command = process.argv[2] || '--generate'
const file = process.argv[3] || path.join('/tmp', `babyeng-device-evidence-${new Date().toISOString().slice(0, 10)}.json`)
const prompts = ['杯子', '碗', '勺子', '叉子', '盘子', '苹果', '牛奶', '面包', '鸡蛋', '香蕉', '球', '小汽车', '帽子', '鞋子', '袜子', '妈妈', '爸爸', '宝宝', '开心', '睡觉了']

function assert(condition, message) {
  if (!condition) throw new Error(message)
}

function device(platform, browser) {
  return {
    platform,
    device_model: null,
    device_class: platform === 'android' ? 'mid_range' : 'physical_device',
    os_version: null,
    browser,
    browser_version: null,
    connection: 'trusted_lan_https',
    tester: null,
    tested_at: null,
  }
}

function checks(names) {
  return Object.fromEntries(names.map((name) => [name, { passed: null, notes: '' }]))
}

function template() {
  return {
    format: 'babyeng-device-evidence-v1',
    generated_at: new Date().toISOString(),
    manual_complete: false,
    a1: {
      requirement: '局域网中端安卓机连续 20 次库内语音提问；总时延 P95 ≤ 1200ms，每次首次界面变化 ≤ 800ms。',
      device: device('android', 'Chrome'),
      samples: prompts.map((prompt, index) => ({ run: index + 1, prompt, first_ui_change_ms: null, total_response_ms: null, expected_word_returned: null, notes: '' })),
    },
    a5: {
      requirement: '安卓 Chrome 与 iOS Safari 分别完成五项录音闭环。',
      platforms: [
        { device: device('android', 'Chrome'), checks: checks(['tap_recording', 'vad_auto_stop', 'dual_track_playback', 'mother_mark_saved', 'under_500ms_not_stored']) },
        { device: device('ios', 'Safari'), checks: checks(['tap_recording', 'vad_auto_stop', 'dual_track_playback', 'mother_mark_saved', 'under_500ms_not_stored']) },
      ],
    },
    a7: {
      requirement: '移动端人工确认服务故障提示、文字兜底及恢复体验。',
      platforms: [
        { device: device('android', 'Chrome'), checks: checks(['tts_unavailable_prompt_visible', 'tts_text_result_retained', 'asr_unavailable_prompt_visible', 'asr_text_input_available', 'ui_recovers_after_services']) },
        { device: device('ios', 'Safari'), checks: checks(['tts_unavailable_prompt_visible', 'tts_text_result_retained', 'asr_unavailable_prompt_visible', 'asr_text_input_available', 'ui_recovers_after_services']) },
      ],
    },
    a8: {
      requirement: '安卓 Chrome 与 iOS Safari 各完成完整闭环，iOS 四项 PWA 限制逐条实测。',
      platforms: [
        { device: device('android', 'Chrome'), full_cycle: checks(['login', 'voice_ask', 'result_card', 'standard_audio', 'child_recording', 'dual_replay', 'mother_mark', 'daily_report']) },
        { device: device('ios', 'Safari'), full_cycle: checks(['login', 'voice_ask', 'result_card', 'standard_audio', 'child_recording', 'dual_replay', 'mother_mark', 'daily_report']) },
      ],
      ios_pwa_limits: [
        { key: 'audio_user_gesture', result: null, observed_behavior: '', fallback_verified: null },
        { key: 'media_recorder_mp4_aac', result: null, observed_behavior: '', observed_mime: '', fallback_verified: null },
        { key: 'background_audio', result: null, observed_behavior: '', fallback_verified: null },
        { key: 'storage_reclamation', result: null, observed_behavior: '', fallback_verified: null },
      ],
    },
    screen_time: {
      requirement: '两平台确认仅前台累计、15 秒批量写入、单次/每日柔性收尾、切页与弱网不重复计时。',
      platforms: [
        { device: device('android', 'Chrome'), checks: checks(['foreground_only', 'batch_write_15s', 'session_limit_wrapup', 'daily_limit_wrapup', 'navigation_no_duplicate', 'weak_network_retry_no_duplicate']) },
        { device: device('ios', 'Safari'), checks: checks(['foreground_only', 'batch_write_15s', 'session_limit_wrapup', 'daily_limit_wrapup', 'navigation_no_duplicate', 'weak_network_retry_no_duplicate']) },
      ],
    },
  }
}

function requireDevice(value, label) {
  assert(value && value.platform && value.device_model && value.os_version && value.browser && value.browser_version, `${label} 缺少真机或浏览器信息`)
  assert(value.tester && value.tested_at, `${label} 缺少验收人或时间`)
  assert(value.connection === 'trusted_lan_https', `${label} 必须使用可信局域网 HTTPS`)
}

function requireChecks(value, label) {
  for (const [key, check] of Object.entries(value || {})) {
    assert(check.passed === true, `${label}.${key} 未通过`)
  }
}

function percentile95(values) {
  const sorted = [...values].sort((left, right) => left - right)
  return sorted[Math.ceil(sorted.length * 0.95) - 1]
}

function requirePlatforms(platforms, label, checkKey) {
  assert(Array.isArray(platforms) && platforms.length === 2, `${label} 必须包含安卓和 iOS 两台真机`)
  assert(new Set(platforms.map((item) => item.device?.platform)).size === 2, `${label} 平台记录重复`)
  assert(platforms.some((item) => item.device?.platform === 'android') && platforms.some((item) => item.device?.platform === 'ios'), `${label} 必须同时包含 android 与 ios`)
  for (const item of platforms) {
    requireDevice(item.device, `${label}.${item.device.platform}`)
    requireChecks(item[checkKey], `${label}.${item.device.platform}`)
  }
}

function verify(evidence) {
  assert(evidence.format === 'babyeng-device-evidence-v1', '真机证据格式不支持')
  requireDevice(evidence.a1?.device, 'A1')
  assert(evidence.a1.device.platform === 'android' && evidence.a1.device.browser === 'Chrome', 'A1 必须使用安卓 Chrome')
  assert(evidence.a1.device.device_class === 'mid_range', 'A1 必须记录为中端安卓机')
  const samples = evidence.a1.samples
  assert(Array.isArray(samples) && samples.length === 20, 'A1 必须恰好记录 20 次')
  assert(samples.every((item, index) => item.run === index + 1 && item.prompt && item.expected_word_returned === true), 'A1 存在未命中或编号不完整的样本')
  assert(samples.every((item) => Number.isFinite(item.first_ui_change_ms) && item.first_ui_change_ms >= 0 && item.first_ui_change_ms <= 800), 'A1 存在首次界面变化超过 800ms 或未填写')
  assert(samples.every((item) => Number.isFinite(item.total_response_ms) && item.total_response_ms >= 0), 'A1 存在未填写的总时延')
  const firstUiP95 = percentile95(samples.map((item) => item.first_ui_change_ms))
  const totalP95 = percentile95(samples.map((item) => item.total_response_ms))
  assert(totalP95 <= 1200, `A1 总时延 P95 ${totalP95}ms 超过 1200ms`)

  requirePlatforms(evidence.a5?.platforms, 'A5', 'checks')
  requirePlatforms(evidence.a7?.platforms, 'A7', 'checks')
  requirePlatforms(evidence.a8?.platforms, 'A8', 'full_cycle')
  requirePlatforms(evidence.screen_time?.platforms, 'screen_time', 'checks')

  const limits = evidence.a8.ios_pwa_limits
  assert(Array.isArray(limits) && limits.length === 4, 'A8 必须记录 iOS 四项 PWA 限制')
  const expectedLimits = ['audio_user_gesture', 'media_recorder_mp4_aac', 'background_audio', 'storage_reclamation']
  assert(expectedLimits.every((key) => limits.some((item) => item.key === key)), 'A8 iOS 限制项目不完整')
  for (const item of limits) {
    assert(['supported', 'limited', 'unavailable'].includes(item.result), `A8.${item.key} 缺少实测结论`)
    assert(item.observed_behavior.trim(), `A8.${item.key} 缺少观察记录`)
    assert(item.fallback_verified === true, `A8.${item.key} 未验证应对路径`)
  }
  const mime = limits.find((item) => item.key === 'media_recorder_mp4_aac')?.observed_mime || ''
  assert(mime.includes('mp4') || mime.includes('aac'), 'A8 未记录 iOS MediaRecorder 的 mp4/aac MIME')

  return { a1_samples: 20, a1_first_ui_p95_ms: firstUiP95, a1_total_p95_ms: totalP95, a5_platforms: 2, a7_platforms: 2, a8_platforms: 2, ios_limits: 4, screen_time_platforms: 2, manual_complete: true }
}

function generate() {
  const output = path.resolve(file)
  fs.mkdirSync(path.dirname(output), { recursive: true })
  fs.writeFileSync(output, JSON.stringify(template(), null, 2) + '\n', { mode: 0o600 })
  fs.chmodSync(output, 0o600)
  console.log(JSON.stringify({ output, manual_complete: false }, null, 2))
}

function selfTest() {
  let blankRejected = false
  try { verify(template()) } catch { blankRejected = true }
  assert(blankRejected, '空真机模板不应通过校验')

  const evidence = template()
  const completeDevice = (value) => Object.assign(value, { device_model: 'physical-test-device', os_version: 'test-os', browser_version: 'test-browser', tester: 'test-reviewer', tested_at: '2026-08-21T00:00:00Z' })
  completeDevice(evidence.a1.device)
  evidence.a1.samples.forEach((sample) => Object.assign(sample, { first_ui_change_ms: 100, total_response_ms: 1000, expected_word_returned: true }))
  for (const section of [evidence.a5, evidence.a7, evidence.screen_time]) {
    section.platforms.forEach((platform) => {
      completeDevice(platform.device)
      Object.values(platform.checks).forEach((check) => { check.passed = true })
    })
  }
  evidence.a8.platforms.forEach((platform) => {
    completeDevice(platform.device)
    Object.values(platform.full_cycle).forEach((check) => { check.passed = true })
  })
  evidence.a8.ios_pwa_limits.forEach((limit) => Object.assign(limit, { result: 'limited', observed_behavior: 'self-test observation', fallback_verified: true }))
  evidence.a8.ios_pwa_limits.find((limit) => limit.key === 'media_recorder_mp4_aac').observed_mime = 'audio/mp4'
  const result = verify(evidence)
  assert(result.manual_complete === true && result.a1_total_p95_ms === 1000, '完整真机证据自测未通过')
  console.log(JSON.stringify({ blank_template_rejected: true, complete_fixture_accepted: true }, null, 2))
}

try {
  if (command === '--generate') generate()
  else if (command === '--verify') console.log(JSON.stringify(verify(JSON.parse(fs.readFileSync(file, 'utf8'))), null, 2))
  else if (command === '--self-test') selfTest()
  else throw new Error('用法：device-evidence.js --generate <文件>、--verify <文件> 或 --self-test')
} catch (error) {
  console.error(error.message)
  process.exit(1)
}
