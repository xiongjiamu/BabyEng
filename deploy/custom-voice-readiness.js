#!/usr/bin/env node
'use strict'

const fs = require('node:fs')
const path = require('node:path')

const FORMAT = 'babyeng-custom-voice-readiness-v1'

function fail(message) {
  throw new Error(message)
}

function assert(condition, message) {
  if (!condition) fail(message)
}

function template() {
  return {
    format: FORMAT,
    generated_at: new Date().toISOString(),
    manual_complete: false,
    scope: 'adult_mother_local_family_english_tts',
    speaker_consent: {
      adult_speaker_confirmed: null,
      speaker_is_rights_holder: null,
      child_voice_excluded: null,
      local_family_use_only: null,
      synthetic_voice_risk_acknowledged: null,
      cloud_training_allowed: false,
      speaker_name: '',
      consent_version: 'custom-voice-consent-v1',
      consented_at: '',
    },
    collection_plan: {
      language: 'en-US',
      target_prompt_count: 120,
      target_usable_minutes: 25,
      holdout_prompt_count: 10,
      exact_transcript_review_required: true,
      third_party_voice_rejection_required: true,
      owner_approved: null,
    },
    deletion_plan: {
      confirmation_text: 'DELETE_CUSTOM_VOICE',
      raw_and_derived_delete: null,
      checkpoints_and_model_delete: null,
      synthesis_cache_delete: null,
      family_binding_fallback: null,
      backup_reappearance_acknowledged: null,
      owner_approved: null,
    },
    training_plan: {
      engine: 'OHF-Voice/piper1-gpl',
      separate_machine: null,
      gpu_model: '',
      vram_gb: null,
      trainer_license_reviewed: null,
      checkpoint_license_reviewed: null,
      owner_approved: null,
    },
    decision: {
      approved_for_collection: false,
      approved_for_training: false,
      approved_by: '',
      approved_at: '',
    },
  }
}

function nonEmpty(value) {
  return typeof value === 'string' && value.trim().length > 0
}

function validDate(value) {
  return nonEmpty(value) && !Number.isNaN(Date.parse(value))
}

function verify(data) {
  assert(data && data.format === FORMAT, `format 必须为 ${FORMAT}`)
  assert(data.manual_complete === true, 'manual_complete 必须由负责人明确改为 true')
  assert(data.scope === 'adult_mother_local_family_english_tts', 'scope 不允许扩展到幼儿或其他用途')

  const consent = data.speaker_consent || {}
  for (const field of ['adult_speaker_confirmed', 'speaker_is_rights_holder', 'child_voice_excluded', 'local_family_use_only', 'synthetic_voice_risk_acknowledged']) {
    assert(consent[field] === true, `speaker_consent.${field} 必须为 true`)
  }
  assert(consent.cloud_training_allowed === false, '当前阶段 cloud_training_allowed 必须为 false')
  assert(nonEmpty(consent.speaker_name), '必须填写成年说话人姓名')
  assert(consent.consent_version === 'custom-voice-consent-v1', '授权版本不匹配')
  assert(validDate(consent.consented_at), '必须填写有效授权时间')

  const collection = data.collection_plan || {}
  assert(collection.language === 'en-US', '当前试采语言必须为 en-US')
  assert(Number.isInteger(collection.target_prompt_count) && collection.target_prompt_count >= 120, '目标提示词不得少于 120 条')
  assert(Number.isFinite(collection.target_usable_minutes) && collection.target_usable_minutes >= 20, '目标可用净语音不得少于 20 分钟')
  assert(Number.isInteger(collection.holdout_prompt_count) && collection.holdout_prompt_count >= 10, '留出试听句不得少于 10 条')
  assert(collection.exact_transcript_review_required === true, '必须要求逐条文本核对')
  assert(collection.third_party_voice_rejection_required === true, '必须拒绝含第三方声音的条目')
  assert(collection.owner_approved === true, '采集方案尚未批准')

  const deletion = data.deletion_plan || {}
  assert(deletion.confirmation_text === 'DELETE_CUSTOM_VOICE', '删除确认文本不匹配')
  for (const field of ['raw_and_derived_delete', 'checkpoints_and_model_delete', 'synthesis_cache_delete', 'family_binding_fallback', 'backup_reappearance_acknowledged', 'owner_approved']) {
    assert(deletion[field] === true, `deletion_plan.${field} 必须为 true`)
  }

  const training = data.training_plan || {}
  assert(training.engine === 'OHF-Voice/piper1-gpl', '训练器版本边界未确认')
  assert(training.separate_machine === true, '训练必须使用与生产服务隔离的机器')
  assert(nonEmpty(training.gpu_model), '必须填写实际 GPU 型号')
  assert(Number.isFinite(training.vram_gb) && training.vram_gb >= 8, '试验 GPU 显存不得低于 8 GB')
  assert(training.trainer_license_reviewed === true, '训练器 GPL-3.0 许可证尚未审核')
  assert(training.checkpoint_license_reviewed === true, '微调 checkpoint 模型卡许可证尚未审核')
  assert(training.owner_approved === true, '训练方案尚未批准')

  const decision = data.decision || {}
  assert(decision.approved_for_collection === true, '尚未批准采集')
  assert(decision.approved_for_training === true, '尚未批准训练')
  assert(nonEmpty(decision.approved_by), '必须填写批准人')
  assert(validDate(decision.approved_at), '必须填写有效批准时间')
  return { readiness_complete: true, format: FORMAT }
}

function generate(file) {
  const target = path.resolve(file)
  assert(!fs.existsSync(target), `拒绝覆盖已有文件：${target}`)
  fs.writeFileSync(target, `${JSON.stringify(template(), null, 2)}\n`, { mode: 0o600, flag: 'wx' })
  fs.chmodSync(target, 0o600)
  return { generated: target, mode: '0600', readiness_complete: false }
}

function completeFixture() {
  const value = template()
  value.manual_complete = true
  Object.assign(value.speaker_consent, {
    adult_speaker_confirmed: true,
    speaker_is_rights_holder: true,
    child_voice_excluded: true,
    local_family_use_only: true,
    synthetic_voice_risk_acknowledged: true,
    speaker_name: 'Adult Speaker',
    consented_at: '2026-08-21T00:00:00Z',
  })
  value.collection_plan.owner_approved = true
  Object.assign(value.deletion_plan, {
    raw_and_derived_delete: true,
    checkpoints_and_model_delete: true,
    synthesis_cache_delete: true,
    family_binding_fallback: true,
    backup_reappearance_acknowledged: true,
    owner_approved: true,
  })
  Object.assign(value.training_plan, {
    separate_machine: true,
    gpu_model: 'verified-test-gpu',
    vram_gb: 8,
    trainer_license_reviewed: true,
    checkpoint_license_reviewed: true,
    owner_approved: true,
  })
  Object.assign(value.decision, {
    approved_for_collection: true,
    approved_for_training: true,
    approved_by: 'Family Owner',
    approved_at: '2026-08-21T00:00:00Z',
  })
  return value
}

function selfTest() {
  let blankRejected = false
  try { verify(template()) } catch { blankRejected = true }
  assert(blankRejected, '空白清单必须被拒绝')
  assert(verify(completeFixture()).readiness_complete, '完整构造清单应通过')
  const unsafe = completeFixture()
  unsafe.speaker_consent.child_voice_excluded = false
  let childVoiceRejected = false
  try { verify(unsafe) } catch { childVoiceRejected = true }
  assert(childVoiceRejected, '包含幼儿音色的清单必须被拒绝')
  return { blank_template_rejected: true, complete_fixture_accepted: true, child_voice_scope_rejected: true }
}

function main(argv) {
  if (argv[0] === '--generate' && argv[1] && argv.length === 2) return generate(argv[1])
  if (argv[0] === '--verify' && argv[1] && argv.length === 2) return verify(JSON.parse(fs.readFileSync(path.resolve(argv[1]), 'utf8')))
  if (argv[0] === '--self-test' && argv.length === 1) return selfTest()
  fail('用法：custom-voice-readiness.js --generate <json> | --verify <json> | --self-test')
}

try {
  process.stdout.write(`${JSON.stringify(main(process.argv.slice(2)), null, 2)}\n`)
} catch (error) {
  process.stderr.write(`${error.message}\n`)
  process.exitCode = 1
}
