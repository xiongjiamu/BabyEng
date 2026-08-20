<template>
  <div class="page">
    <header class="appbar">
      <span class="icon-btn" aria-hidden="true"></span>
      <div class="dots">
        <i v-for="(_, i) in 3" :key="i" :class="{ on: i < step }"></i>
      </div>
      <button class="icon-btn" style="font-size:14px;font-weight:700;width:auto;padding:0 12px" @click="skip">
        跳过
      </button>
    </header>

    <!-- 第 1 步：孩子生日（PRD 6.7：问生日定分段） -->
    <section v-show="step === 1" class="page-body pad stack-6">
      <div class="hero-ico">👶</div>
      <div class="stack-3 center-text">
        <h2 class="t-zh-lg">宝宝是哪年哪月出生的？</h2>
        <p class="t-mom">我们会按月龄调整跟读方式。</p>
      </div>
      <div class="picker">
        <select v-model="year">
          <option v-for="y in years" :key="y" :value="y">{{ y }} 年</option>
        </select>
        <select v-model="month">
          <option v-for="m in 12" :key="m" :value="m">{{ m }} 月</option>
        </select>
      </div>
      <div class="derived" v-html="derivedHtml"></div>
      <button class="btn btn-primary btn-block btn-lg" @click="step = 2">下一步</button>
      <p class="note">跳过则默认按 B 段处理，并在设置页顶部保留一条待补提示。分段随时可在设置里手动覆盖。</p>
    </section>

    <!-- 第 2 步：麦克风权限（6.7：先讲清用途，被拒不反复弹窗） -->
    <section v-show="step === 2" class="page-body pad stack-6">
      <div class="hero-ico">🎙</div>
      <div class="stack-3 center-text">
        <h2 class="t-zh-lg">需要用一下麦克风</h2>
        <p class="t-mom">用来听你说中文，以及录下宝宝的跟读。</p>
      </div>
      <ul class="perm-list" style="list-style:none;padding:0;margin:0">
        <li><span class="ico">🔒</span><span>录音只存在你自己的服务器上，不上传任何地方</span></li>
        <li><span class="ico">🗓</span><span>默认保留 30 天，喜欢的可以收藏长期留着</span></li>
        <li><span class="ico">✍️</span><span>不给权限也能用，改成打字提问</span></li>
      </ul>
      <div class="stack-3">
        <button class="btn btn-primary btn-block btn-lg" @click="requestMic(true)">允许使用麦克风</button>
        <button class="btn-quiet btn-block" @click="requestMic(false)">先用打字，以后再说</button>
      </div>
    </section>

    <!-- 第 3 步：示范一次（手把手：问→出卡→播发音→跟读录音→回放） -->
    <section v-show="step === 3" class="page-body pad stack-6">
      <div v-if="demo === 1" class="stack-6" style="align-items:center">
        <div class="stack-3 center-text">
          <h2 class="t-zh-lg">我们一起试一次</h2>
          <p class="t-mom">走完这一轮，你就已经会用了。</p>
        </div>
        <div class="coach">按住话筒，说「杯子」</div>
        <button class="record record-mom" @pointerdown="demo = 2" aria-label="按住说话"><span class="ico">🎙</span></button>
      </div>

      <div v-else-if="demo === 2" class="stack-5 enter">
        <div class="stack-3 center-text">
          <h2 class="t-zh-lg">我们一起试一次</h2>
          <p class="t-mom">走完这一轮，你就已经会用了。</p>
        </div>
        <div class="photo" style="width:150px;margin:0 auto"><span class="emoji">☕</span></div>
        <div class="demo-word">
          <div class="t-word-en">cup</div>
          <div class="t-phonetic">/kʌp/</div>
          <div class="t-zh" style="color:var(--c-ink-2);margin-top:var(--sp-2)">杯子</div>
        </div>
        <div class="coach">点一下听发音，你先自己念一遍</div>
        <button class="btn btn-primary btn-block btn-lg" @click="playDemo">▶ 播放发音</button>
      </div>

      <div v-else-if="demo === 3" class="stack-5 enter">
        <div class="demo-word">
          <div class="t-word-en">cup</div>
          <div class="t-phonetic">/kʌp/</div>
        </div>
        <div class="coach">现在让宝宝跟着说一次</div>
        <div style="display:flex;justify-content:center">
          <button class="record" @click="demoRecord" aria-label="点一下开始录音"><span class="ico">🎤</span></button>
        </div>
        <p class="t-mom center-text">点一下开始，安静 1.5 秒自动停</p>
      </div>

      <div v-else class="stack-6 enter">
        <div class="hero-ico">🎉</div>
        <div class="stack-3 center-text">
          <h2 class="t-zh-lg">就是这样，你已经会了</h2>
          <p class="t-mom">看到什么就问什么。<br>每天几个词，比一次学一堆管用。</p>
        </div>
        <button class="btn btn-primary btn-block btn-lg" @click="finish">开始用吧</button>
      </div>
    </section>
  </div>
</template>

<script setup>
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { useAppStore } from '../stores/app'
import { api } from '../api'
import { useAudio } from '../composables/useAudio'

const router = useRouter()
const store = useAppStore()
const { playUrl } = useAudio()

const step = ref(1)
const demo = ref(1)
const year = ref(new Date().getFullYear() - 2)
const month = ref(10)
const years = computed(() => {
  const y = new Date().getFullYear()
  return [y - 3, y - 2, y - 1, y]
})

// 月龄推导（PRD 1.2：12~24 月 A 段，24~36 月 B 段）
const derived = computed(() => {
  const now = new Date()
  const bd = new Date(year.value, month.value - 1, 1)
  let months = (now.getFullYear() - bd.getFullYear()) * 12 + (now.getMonth() - bd.getMonth())
  if (now.getDate() < bd.getDate()) months -= 1
  return { months, band: months >= 12 && months < 24 ? 'A' : months >= 24 && months < 36 ? 'B' : null }
})
const derivedHtml = computed(() => {
  const d = derived.value
  if (!d.band) {
    return `现在 <b>${d.months} 个月</b> · <b>比建议年龄${d.months < 12 ? '小' : '大'}</b><br><span class="t-mom-sm">${d.months < 12 ? '可以先只用纯音频模式给宝宝听，屏幕相关功能会一直关着。' : '已经超出 36 月范围，按 B 段交互处理。'}</span>`
  }
  if (d.band === 'A') {
    return `现在 <b>${d.months} 个月</b> · 归入 <b>A 段</b><br><span class="t-mom-sm">宝宝不用碰手机，全程你来操作；默认开纯音频模式，宝宝只听声音。</span>`
  }
  return `现在 <b>${d.months} 个月</b> · 归入 <b>B 段</b><br><span class="t-mom-sm">宝宝可以自己点大按钮、做听音选图；单次 5 分钟，每天上限 15 分钟。</span>`
})

onMounted(async () => {
  await store.bootstrap()
})

async function requestMic(allow) {
  if (allow) {
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true })
      stream.getTracks().forEach((t) => t.stop())
      store.setMicPermission('granted')
    } catch {
      store.setMicPermission('denied')
    }
  } else {
    store.setMicPermission('denied')
  }
  step.value = 3
}

function playDemo() {
  playUrl(api.ttsUrl('cup', 0.8, store.settings.ttsVoice), { rate: 0.8 })
  demo.value = 3
}

async function demoRecord() {
  // 演示录音：模拟点按→静音自动停
  demo.value = 4
}

async function finish() {
  const birthdate = `${year.value}-${String(month.value).padStart(2, '0')}-01`
  try {
    await store.completeOnboarding({ child_name: '宝宝', child_birthdate: birthdate })
  } catch (e) {
    // 网络不可用时本地兜底，仍可进入首页
    localStorage.setItem('babyeng_initialized', '1')
    store.initialized = true
  }
  router.replace('/home')
}

function skip() {
  // 跳过生日 → 默认 B 段（6.7）
  finish()
}
</script>
