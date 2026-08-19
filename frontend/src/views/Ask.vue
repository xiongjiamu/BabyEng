<template>
  <div class="page">
    <header class="appbar">
      <router-link class="icon-btn" to="/home" aria-label="返回首页">←</router-link>
      <h1>问一问</h1>
      <router-link class="icon-btn" to="/settings" aria-label="设置">⚙</router-link>
    </header>

    <!-- 麦克风权限被拒常驻提示（5.4） -->
    <div v-if="store.micPermission === 'denied'" class="device-banner" style="padding:0 var(--sp-5)">
      <div class="banner warn">
        <span class="ico">🎙</span>
        <span class="grow"><b>麦克风没打开</b>，现在只能打字提问</span>
        <button class="chip" style="background:#fff" @click="goSettings">去开启</button>
      </div>
    </div>
    <!-- 非 HTTPS（5.4：浏览器只在加密地址下允许麦克风） -->
    <div v-if="!isSecure" class="device-banner" style="padding:0 var(--sp-5)">
      <div class="banner danger">
        <span class="ico">🔓</span>
        <span class="grow"><b>这个地址没法录音</b>，请用 https 地址打开</span>
      </div>
    </div>

    <!-- ============ 1. 待提问 ============ -->
    <section v-show="state === 'idle'" class="page-body pad center stack-8">
      <div class="stack-4" style="text-align:center">
        <p class="ask-hint">按住下面的按钮<br>说出你想教的东西</p>
        <p class="t-mom">比如「杯子英语怎么说」「爷爷」「该睡觉了」</p>
      </div>
      <div class="stack-4" style="align-items:center">
        <button
          class="record record-mom"
          :class="{ 'is-recording': recording }"
          @pointerdown="startAsk"
          @pointerup="stopAsk"
          aria-label="按住说话"
        >
          <span v-if="!recording" class="ico">🎙</span>
          <span v-else class="wave"><i></i><i></i><i></i><i></i><i></i></span>
        </button>
        <span class="chip mom">妈妈按住说 · 松开就查</span>
      </div>
      <button class="btn-quiet" @click="showTextInput = true">改用打字 →</button>
      <div v-if="showTextInput" class="textinput enter" style="width:100%">
        <input v-model="textInput" type="text" placeholder="输入中文，如 杯子" @keyup.enter="askByText" />
        <button class="btn btn-mom" @click="askByText">查</button>
      </div>
    </section>

    <!-- ============ 2. 录音中 ============ -->
    <section v-show="state === 'recording'" class="page-body pad center stack-8">
      <p class="ask-hint">在听……</p>
      <div class="stack-4" style="align-items:center">
        <button class="record record-mom is-recording" aria-label="松开结束">
          <span class="wave"><i></i><i></i><i></i><i></i><i></i></span>
        </button>
        <span class="chip mom">{{ formatTime(askDuration) }} · 松开结束</span>
      </div>
      <p class="t-mom center-text">母亲侧用按住式（press-to-talk，PRD 6.1）</p>
    </section>

    <!-- ============ 3. 识别中（800ms 内必须回显） ============ -->
    <section v-show="state === 'recognizing'" class="page-body pad stack-6">
      <div class="heard enter">
        <span>正在听……{{ recognizedHint }}</span>
      </div>
      <div class="photo result-photo" style="opacity:.35">
        <span class="emoji">🔍</span>
      </div>
      <p class="ask-hint">正在查……</p>
      <div class="progress"><i style="width:62%"></i></div>
      <p class="note">PRD 4.1.3：识别文本必须在 800ms 内回显，库内命中整条链路 P95 ≤ 1.2s。</p>
    </section>

    <!-- ============ 4. 结果卡（库内命中） ============ -->
    <section v-if="state === 'result' && result" class="page-body pad stack-5 enter">
      <div class="heard">
        <span>听到：{{ recognizedText }}</span>
        <button @click="reset">不对？</button>
      </div>

      <div v-if="result.image_emoji" class="photo result-photo"><span class="emoji">{{ result.image_emoji }}</span></div>

      <div class="word-block">
        <div class="t-word-en">{{ result.en }}</div>
        <div v-if="result.phonetic" class="t-phonetic">{{ result.phonetic }}</div>
        <div class="t-zh">{{ result.zh }}</div>
      </div>

      <div class="play-row">
        <button class="play-big" @click="playResult(1)">
          <span>▶</span><span>{{ playing ? '播放中' : '播放发音' }}</span>
        </button>
        <button class="play-slow" aria-label="慢速播放" @click="playResult(0.6)">0.6×</button>
      </div>

      <div v-if="result.example_en" class="example">
        <div class="en">{{ result.example_en }}</div>
        <div class="zh">{{ result.example_zh }}</div>
      </div>

      <div v-if="result.mother_tip" class="card-flat stack-3">
        <div class="t-label">妈妈学习卡</div>
        <p class="t-mom" style="margin:0">{{ result.mother_tip }}</p>
      </div>

      <p v-if="result.match_level" class="note" style="margin:0">匹配 {{ result.match_level }} · 音标来源 {{ result.phonetic_source || 'dict' }}（PRD 4.1.2：音标只取词典与 g2p，永不采信 LLM）</p>
    </section>

    <!-- ============ 5. 识别歧义：二选一 ============ -->
    <section v-show="state === 'ambiguous'" class="page-body pad stack-6 enter">
      <div class="heard"><span>听到：{{ recognizedText }}</span></div>
      <h2 class="t-zh-lg center-text">你是说哪一个？</h2>
      <div class="choice">
        <button v-for="c in candidates" :key="c.target_id" @click="confirm(c)">
          <span class="photo sm"><span class="emoji">{{ c.image_emoji || '❓' }}</span></span>
          <span class="grow">
            <div class="t-word-en-s">{{ c.en }}</div>
            <div class="t-mom">{{ c.zh }}{{ c.scene ? ' · ' + sceneLabel(c.scene) : '' }}</div>
          </span>
        </button>
      </div>
      <button class="btn-quiet" @click="reset">都不是，重新说</button>
      <p class="note">PRD 4.1.1 L2 层：拼音相似度接近阈值时不猜，交给母亲一次点选。</p>
    </section>

    <!-- ============ 6. 彻底未命中 ============ -->
    <section v-show="state === 'nomatch'" class="page-body pad stack-5 enter">
      <div class="heard"><span>听到：{{ recognizedText }}</span></div>
      <div class="banner warn">
        <span class="ico">💡</span>
        <span>这个词还没准备好，我记下了。<br>先试试下面这些，或者打字告诉我。</span>
      </div>

      <div>
        <div class="t-label" style="margin-bottom:var(--sp-3)">同类目，宝宝也常听到</div>
        <div class="suggest">
          <button v-for="c in candidates" :key="c.target_id" @click="confirm(c)">
            <span class="photo sm"><span class="emoji">{{ c.image_emoji || '❓' }}</span></span>
            <span class="t-word-en-s">{{ c.en }}</span>
            <span class="t-mom-sm">{{ c.zh }}</span>
          </button>
        </div>
      </div>

      <div class="stack-3">
        <div class="t-label">或者打字</div>
        <div class="textinput">
          <input v-model="textInput" type="text" placeholder="输入中文，如 口红" @keyup.enter="askByText" />
          <button class="btn btn-mom" @click="askByText">查</button>
        </div>
      </div>

      <p class="note">这条查询已静默写入未命中表（PRD 8.8）：未命中不是错误页，是词库增长的输入源。</p>
    </section>

    <!-- ============ 7. ASR 无结果 ============ -->
    <section v-show="state === 'asrfail'" class="page-body pad center stack-6 enter">
      <div style="text-align:center">
        <div style="font-size:72px">🔇</div>
        <h2 class="t-zh-lg" style="margin:var(--sp-4) 0 var(--sp-2)">没听清</h2>
        <p class="t-mom">周围有点吵，或者离得远了一点</p>
      </div>
      <div class="stack-3" style="width:100%">
        <button class="btn btn-primary btn-block btn-lg" @click="reset">再说一次</button>
        <div class="textinput">
          <input v-model="textInput" type="text" placeholder="也可以直接打字" @keyup.enter="askByText" />
          <button class="btn btn-mom" @click="askByText">查</button>
        </div>
      </div>
      <p class="note">PRD 6.1：连续两次识别失败，文字输入框主动展开，不让母亲自己去找。</p>
    </section>

    <!-- ============ 8. TTS 不可用（降级但不阻断） ============ -->
    <section v-if="state === 'ttsdown' && result" class="page-body pad stack-5 enter">
      <div class="banner danger">
        <span class="ico">🔈</span>
        <span>发音暂时不可用，其他都能用。<br>可以先看着音标教，稍后自动恢复。</span>
      </div>
      <div v-if="result.image_emoji" class="photo result-photo"><span class="emoji">{{ result.image_emoji }}</span></div>
      <div class="word-block">
        <div class="t-word-en">{{ result.en }}</div>
        <div v-if="result.phonetic" class="t-phonetic">{{ result.phonetic }}</div>
        <div class="t-zh">{{ result.zh }}</div>
      </div>
      <div class="play-row">
        <button class="play-big" aria-disabled="true" style="background:var(--c-surface-2);color:var(--c-ink-3);box-shadow:none">
          <span>▶</span><span>发音暂时不可用</span>
        </button>
      </div>
      <p class="note">PRD 4.1.3 / 9.10：TTS 挂掉只降级发音一项，文字结果照出，跟读录音照常可用。</p>
    </section>

    <!-- 底部动作条：结果态才出现 -->
    <div v-if="state === 'result' || state === 'ttsdown'" class="footer-act">
      <div class="stack-3">
        <router-link class="btn btn-primary btn-block btn-lg" :to="{ path: '/compare', query: { target_type: result?.target_type, target_id: result?.target_id, en: result?.en, zh: result?.zh, phonetic: result?.phonetic, emoji: result?.image_emoji } }">
          🎤 让宝宝跟读
        </router-link>
        <button class="btn-quiet btn-block" @click="reset">再问一个</button>
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { useAppStore } from '../stores/app'
import { api } from '../api'
import { useAudio } from '../composables/useAudio'

const router = useRouter()
const store = useAppStore()
const { unlock, playUrl } = useAudio()

const state = ref('idle')
const result = ref(null)
const candidates = ref([])
const recognizedText = ref('')
const recognizedHint = ref('')
const recording = ref(false)
const askDuration = ref(0)
const showTextInput = ref(false)
const textInput = ref('')
const playing = ref(false)

const isSecure = computed(() => window.isSecureContext)

let askStream = null
let askRecorder = null
let askChunks = []
let askTimer = null
let askStartTs = 0

onMounted(async () => {
  await store.bootstrap()
  if (!store.initialized) {
    location.href = '/onboarding'
    return
  }
  // 录音权限状态
  try {
    const s = await navigator.mediaDevices.getUserMedia({ audio: true })
    s.getTracks().forEach((t) => t.stop())
    store.setMicPermission('granted')
  } catch {
    store.setMicPermission('denied')
  }
})

onBeforeUnmount(() => {
  if (askTimer) clearInterval(askTimer)
  askStream?.getTracks().forEach((t) => t.stop())
})

function formatTime(ms) {
  const s = Math.floor(ms / 1000)
  return `0:${String(s).padStart(2, '0')}`
}

// ---------- 母亲按住式录音（6.1） ----------
async function startAsk() {
  if (recording.value) return
  // 解锁音频上下文（iOS：首次播放必须手势触发，PRD 9.2）
  unlock()
  try {
    askStream = await navigator.mediaDevices.getUserMedia({
      audio: { echoCancellation: true, noiseSuppression: true },
    })
    const mime = pickMime()
    askRecorder = new MediaRecorder(askStream, mime ? { mimeType: mime } : undefined)
    askChunks = []
    askRecorder.ondataavailable = (e) => { if (e.data.size) askChunks.push(e.data) }
    askRecorder.start(100)
    recording.value = true
    state.value = 'recording'
    askStartTs = Date.now()
    askDuration.value = 0
    askTimer = setInterval(() => { askDuration.value = Date.now() - askStartTs }, 100)
  } catch {
    store.setMicPermission('denied')
  }
}

async function stopAsk() {
  if (!recording.value) return
  recording.value = false
  if (askTimer) clearInterval(askTimer)
  const duration = Date.now() - askStartTs
  if (duration < 400) {
    // 误触
    reset()
    return
  }
  askRecorder.onstop = async () => {
    askStream?.getTracks().forEach((t) => t.stop())
    askStream = null
    const blob = new Blob(askChunks, { type: askRecorder.mimeType || 'audio/webm' })
    askChunks = []
    await recognizeAudio(blob)
  }
  askRecorder.stop()
}

function pickMime() {
  const cands = ['audio/webm;codecs=opus', 'audio/webm', 'audio/mp4', 'audio/ogg;codecs=opus']
  for (const m of cands) if (MediaRecorder.isTypeSupported(m)) return m
  return ''
}

function blobExt(mime) {
  if (!mime) return 'webm'
  if (mime.includes('mp4') || mime.includes('aac')) return 'm4a'
  if (mime.includes('ogg')) return 'ogg'
  return 'webm'
}

// ---------- ASR + 匹配 ----------
async function recognizeAudio(blob) {
  state.value = 'recognizing'
  try {
    const res = await api.askVoice(blob, `ask.${blobExt(blob.mimeType || '')}`, store.childId, store.familyId)
    applyResponse(res)
  } catch (e) {
    if (e.code === 'asr_unavailable') {
      state.value = 'asrfail'
      recognizedText.value = ''
    } else {
      state.value = 'nomatch'
    }
  }
}

async function askByText() {
  const t = textInput.value.trim()
  if (!t) return
  textInput.value = ''
  state.value = 'recognizing'
  recognizedHint.value = t
  try {
    const res = await api.askText(t, { child_id: store.childId, family_id: store.familyId })
    applyResponse(res)
  } catch (e) {
    if (e.degradable) {
      state.value = 'asrfail'
      recognizedText.value = t
    } else {
      state.value = 'nomatch'
      recognizedText.value = t
    }
  }
}

function applyResponse(res) {
  if (res.status === 'hit' || res.status === 'tts_only_down') {
    result.value = res.result
    recognizedText.value = res.recognized_text || ''
    // 后端 tts_only_down → 前端 ttsdown 态（显示「发音暂时不可用」）
    state.value = res.status === 'tts_only_down' ? 'ttsdown' : 'result'
    if (res.status === 'hit') {
      // 自动播一次（本次会话音频上下文已解锁，PRD 4.1 第 7 步）
      setTimeout(() => playResult(store.settings.ttsRate), 400)
    }
  } else if (res.status === 'ambiguous') {
    candidates.value = res.candidates
    recognizedText.value = res.recognized_text || ''
    state.value = 'ambiguous'
  } else if (res.status === 'nomatch') {
    candidates.value = res.candidates
    recognizedText.value = res.recognized_text || ''
    store.lastUnmatchedId = res.unmatched_id
    state.value = 'nomatch'
  } else if (res.status === 'asr_fail') {
    state.value = 'asrfail'
    recognizedText.value = res.recognized_text || ''
  }
}

async function confirm(c) {
  try {
    const res = await api.askConfirm(c.target_type, c.target_id, store.childId)
    result.value = res.result
    state.value = res.result?.tts_available === false ? 'ttsdown' : 'result'
  } catch {
    // 本地兜底
    result.value = c
    state.value = 'result'
  }
}

function playResult(rate) {
  if (!result.value) return
  playing.value = true
  playUrl(api.ttsUrl(result.value.en, rate), { rate }).finally(() => {
    playing.value = false
  })
}

function reset() {
  state.value = 'idle'
  result.value = null
  candidates.value = []
  recognizedText.value = ''
  recognizedHint.value = ''
  askChunks = []
}

function sceneLabel(s) {
  return ({ morning: '起床', meal: '吃饭', play: '玩耍', bedtime: '睡前', outing: '出门' })[s] || s
}

function goSettings() {
  router.push('/settings')
}
</script>
