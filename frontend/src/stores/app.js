import { defineStore } from 'pinia'
import { api, ApiError } from '../api'

const DEFAULT_TTS_VOICE = 'en_US-mike-medium'
const TTS_VOICES = new Set([
  DEFAULT_TTS_VOICE,
  'en_US-amy-medium',
  'en_US-ryan-medium',
  'en_US-kristin-medium',
  'en_US-hfc_female-medium',
  'en_US-hfc_male-medium',
])
const BOOTSTRAP_TTL_MS = 5 * 60 * 1000
let bootstrapPromise = null

// 全局状态：家庭/孩子/年龄分段/设置/屏幕时间（PRD 6.5 / 11.3）
export const useAppStore = defineStore('app', {
  state: () => ({
    loading: false,
    lastBootstrapAt: 0,
    initialized: false, // 是否完成首次引导（6.7）
    family: null,
    child: null,
    ageBand: null, // 'A' | 'B'
    ageMonths: null,
    settings: {
      ttsRate: 0.8,
      ttsVoice: DEFAULT_TTS_VOICE,
      audioOnly: true, // A 段默认开启（6.6）
      screenLimitMin: 5, // 分钟/天
      sessionLimitMin: 3,
      bedtimeHour: 21,
      cloudModel: false, // 11.4 知情同意
      cloudConsentedAt: null,
      availableMaterials: [],
      childInterests: [],
    },
    // 会话内屏幕计时（B 段看图时累计，纯音频不计入）
    screenSecToday: 0,
    screenSessionSec: 0,
    // 推理服务就绪状态（首页「正在启动」提示条，5.4）
    svcReady: { tts: false, asr: false, llm: false },
    // 麦克风权限状态
    micPermission: 'unknown', // 'granted' | 'denied' | 'unknown'
    // 未命中提示队列（用于 Ask 页 nomatch 后展示）
    lastUnmatchedId: null,
  }),

  getters: {
    isBandA: (s) => s.ageBand === 'A',
    isBandB: (s) => s.ageBand === 'B',
    dailyScreenLimitSec: (s) => (s.settings.screenLimitMin || 0) * 60,
    screenExceeded: (s) => s.dailyScreenLimitSec > 0 && s.screenSecToday >= s.dailyScreenLimitSec,
    sessionScreenLimitSec: (s) => (s.settings.sessionLimitMin || 0) * 60,
    sessionExceeded: (s) => s.sessionScreenLimitSec > 0 && s.screenSessionSec >= s.sessionScreenLimitSec,
    childId: (s) => s.child?.child_id || localStorage.getItem('babyeng_child_id') || '',
    familyId: (s) => s.family?.family_id || localStorage.getItem('babyeng_family_id') || '',
  },

  actions: {
    async bootstrap({ force = false } = {}) {
      if (!force && this.lastBootstrapAt && Date.now() - this.lastBootstrapAt < BOOTSTRAP_TTL_MS) return
      if (bootstrapPromise) return bootstrapPromise
      // 启动时拉取家庭信息；未初始化 → 引导页
      this.loading = true
      bootstrapPromise = (async () => {
        try {
          const me = await api.familyMe()
          this.lastBootstrapAt = Date.now()
          if (me.initialized) {
            this.initialized = true
            this.family = me.family
            this.child = me.child
            this.ageBand = me.age_band
            this.ageMonths = me.age_months
            const s = me.settings || {}
            this.settings = {
              ttsRate: s.tts_rate ?? 0.8,
              ttsVoice: TTS_VOICES.has(s.tts_voice) ? s.tts_voice : DEFAULT_TTS_VOICE,
              audioOnly: s.audio_only ?? this.ageBand === 'A',
              screenLimitMin: s.screen_limit_min ?? (this.ageBand === 'B' ? 15 : 5),
              sessionLimitMin: s.session_limit_min ?? (this.ageBand === 'B' ? 5 : 3),
              bedtimeHour: s.bedtime_hour ?? 21,
              cloudModel: !!s.cloud_model,
              cloudConsentedAt: s.cloud_consented_at || null,
              availableMaterials: Array.isArray(s.available_materials) ? s.available_materials : [],
              childInterests: Array.isArray(s.child_interests) ? s.child_interests : [],
            }
            if (me.child?.child_id) localStorage.setItem('babyeng_child_id', me.child.child_id)
            if (me.family?.family_id) localStorage.setItem('babyeng_family_id', me.family.family_id)
          }
        } catch (e) {
          if (e?.status === 401) {
            this.resetUserData()
            return
          }
          // 网络失败：本地兜底，保证离线可打开（9.2）
          console.warn('bootstrap 失败，走本地模式', e)
          this.initialized = !!localStorage.getItem('babyeng_initialized')
        } finally {
          this.loading = false
          bootstrapPromise = null
        }
      })()
      return bootstrapPromise
    },

    resetUserData() {
      this.initialized = false
      this.family = null
      this.child = null
      this.ageBand = null
      this.ageMonths = null
      this.lastBootstrapAt = 0
      bootstrapPromise = null
      for (const key of ['babyeng_initialized', 'babyeng_family_id', 'babyeng_child_id']) {
        localStorage.removeItem(key)
      }
    },

    async completeOnboarding(data) {
      const res = await api.familyInit(data)
      localStorage.setItem('babyeng_initialized', '1')
      if (res.family_id) localStorage.setItem('babyeng_family_id', res.family_id)
      if (res.child_id) localStorage.setItem('babyeng_child_id', res.child_id)
      this.initialized = true
      await this.bootstrap({ force: true })
      return res
    },

    setBand(band) {
      this.ageBand = band
      // A 段默认纯音频开、屏幕上限 5 分钟；B 段默认 15 分钟（11.3）
      if (band === 'A') {
        if (!('audioOnlySet' in this.settings)) this.settings.audioOnly = true
        this.settings.screenLimitMin = Math.min(this.settings.screenLimitMin || 5, 5)
      } else {
        this.settings.screenLimitMin = Math.max(this.settings.screenLimitMin || 15, 5)
      }
    },

    async saveSettings(partial) {
      const previous = this.settings
      this.settings = { ...this.settings, ...partial }
      try {
        await api.familySettings({
          tts_rate: this.settings.ttsRate,
          tts_voice: this.settings.ttsVoice,
          audio_only: this.settings.audioOnly,
          screen_limit_min: this.settings.screenLimitMin,
          session_limit_min: this.settings.sessionLimitMin,
          bedtime_hour: this.settings.bedtimeHour,
          cloud_model: this.settings.cloudModel,
          cloud_consented_at: this.settings.cloudConsentedAt,
          available_materials: this.settings.availableMaterials,
          child_interests: this.settings.childInterests,
        })
        return true
      } catch (e) {
        this.settings = previous
        console.warn('设置保存失败', e)
        return false
      }
    },

    // 屏幕时间累计（看图模式才累计；纯音频不计入，6.6）
    tickScreen(seconds) {
      this.screenSecToday += seconds
      this.screenSessionSec += seconds
    },

    setScreenTimeToday(seconds) {
      this.screenSecToday = Math.max(0, Number(seconds) || 0)
    },

    resetScreenSession() {
      this.screenSessionSec = 0
    },

    setMicPermission(p) {
      this.micPermission = p
    },

    async refreshSvcStatus() {
      try {
        const r = await fetch('/api/readyz').then((x) => x.json())
        this.svcReady = r.services || this.svcReady
      } catch {
        /* 忽略 */
      }
    },
  },
})

export { ApiError }
