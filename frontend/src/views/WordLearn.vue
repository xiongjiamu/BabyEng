<template>
  <div class="page">
    <header class="appbar">
      <router-link class="icon-btn" to="/learn" aria-label="退出学习">✕</router-link>
      <h1 style="font-size:17px">{{ categoryName }} · 第 {{ pos }} / {{ total }} 个</h1>
      <span class="icon-btn" aria-hidden="true"></span>
    </header>

    <div class="steps">
      <i v-for="(w, i) in group" :key="w.id" :class="{ done: i < groupIdx, now: i === groupIdx }"></i>
    </div>

    <!-- ============ 逐词流 ============ -->
    <section v-if="view === 'learn' && currentWord" class="page-body pad stack-5">
      <div class="photo" style="width:186px;margin:0 auto"><span class="emoji">{{ currentWord.image_emoji || '🖼' }}</span></div>

      <div style="text-align:center">
        <div class="t-word-en">{{ currentWord.en }}</div>
        <div class="t-phonetic">{{ currentWord.phonetic || '' }}</div>
        <div class="t-zh" style="color:var(--c-ink-2);margin-top:var(--sp-2)">{{ currentWord.zh }}</div>
      </div>

      <div class="flow">
        <div class="flow-step done"><span class="n">1</span><span>看图</span></div>
        <div class="flow-step" :class="{ done: flowStep > 1, now: flowStep === 1 }"><span class="n">2</span><span>听标准音</span></div>
        <div class="flow-step" :class="{ done: flowStep > 2, now: flowStep === 2 }"><span class="n">3</span><span>妈妈先跟读一遍</span></div>
        <div class="flow-step" :class="{ now: flowStep === 3 }"><span class="n">4</span><span>宝宝跟读并录音</span></div>
      </div>

      <div class="row" style="gap:var(--sp-3)">
        <button class="btn btn-ghost" style="flex:none;width:76px;min-height:60px" @click="playCurrent">▶</button>
        <router-link class="btn btn-primary grow" style="min-height:60px" :to="{ path: '/compare', query: { target_type: 'word', target_id: currentWord.id, en: currentWord.en, zh: currentWord.zh, phonetic: currentWord.phonetic, emoji: currentWord.image_emoji } }">
          🎤 让宝宝跟读
        </router-link>
      </div>

      <div class="row-between">
        <button class="btn-quiet" @click="prev" :disabled="groupIdx === 0">‹ 上一个</button>
        <button class="btn-quiet" @click="nextWord">跳过，下一个 ›</button>
      </div>
    </section>

    <!-- ============ B 段组末：听音选图小测 ============ -->
    <section v-else-if="view === 'quiz'" class="page-body pad stack-6 enter">
      <div style="text-align:center">
        <span class="chip kid">小测 · 5 个词学完了</span>
      </div>
      <h2 class="t-zh-lg center-text">哪个是 <span style="font-size:34px">{{ quizWord.en }}</span>？</h2>
      <div style="display:flex;justify-content:center">
        <button class="btn btn-primary btn-lg" style="min-width:180px" @click="playQuiz">🔊 再听一次</button>
      </div>
      <div class="quiz-opts">
        <button v-for="(opt, i) in quizOptions" :key="opt.id" class="quiz-opt" @click="answerQuiz(opt.id === quizWord.id)">
          <span class="photo"><span class="emoji">{{ opt.image_emoji || '🖼' }}</span></span>
        </button>
      </div>
      <p class="note">
        <b>小测只在 B 段（24~36 月）出现</b>，且只做二选一。A 段幼儿无法稳定完成选择题，强行加测只会制造挫败。
      </p>
    </section>

    <!-- ============ A 段组末：直接鼓励，不测 ============ -->
    <section v-else-if="view === 'cheer'" class="page-body pad center stack-6 enter">
      <div style="text-align:center">
        <div style="font-size:88px">🐙</div>
        <h2 class="t-zh-lg" style="margin:var(--sp-4) 0 var(--sp-2)">这一组学完啦</h2>
        <p class="t-mom">今天跟读了 {{ recCount }} 次，都很棒。</p>
      </div>
      <div class="stack-3" style="width:100%">
        <button class="btn btn-primary btn-block btn-lg" @click="nextGroup">再来一组</button>
        <router-link class="btn-quiet btn-block" to="/home">今天先到这儿</router-link>
      </div>
      <p class="note">A 段跳过小测，直接进入组末鼓励。这一屏没有任何分数、等级或对错。</p>
    </section>

    <!-- ============ B 段小测后总结 ============ -->
    <section v-else-if="view === 'summaryB'" class="page-body pad center stack-6 enter">
      <div style="text-align:center">
        <div style="font-size:88px">🎉</div>
        <h2 class="t-zh-lg" style="margin:var(--sp-4) 0 var(--sp-2)">这一组学完啦</h2>
        <p class="t-mom">5 个词，跟读 5 次，小测 1 次。</p>
      </div>
      <div class="card stack-3" style="width:100%">
        <div class="t-label">明天会先复习</div>
        <div class="row">
          <span class="photo sm"><span class="emoji">{{ weakWord.image_emoji || '🖼' }}</span></span>
          <span class="grow"><div class="t-word-en-s">{{ weakWord.en }}</div><div class="t-mom-sm">还没稳，明天再来一次</div></span>
        </div>
      </div>
      <div class="stack-3" style="width:100%">
        <button class="btn btn-primary btn-block btn-lg" @click="nextGroup">再来一组</button>
        <router-link class="btn-quiet btn-block" to="/home">今天先到这儿</router-link>
      </div>
    </section>
  </div>
</template>

<script setup>
import { computed, onMounted, ref } from 'vue'
import { useRoute } from 'vue-router'
import { useAppStore } from '../stores/app'
import { api } from '../api'
import { useAudio } from '../composables/useAudio'

const route = useRoute()
const store = useAppStore()
const { playUrl, unlock } = useAudio()

const category = ref(route.query.category || 'item')
const words = ref([])
const view = ref('learn')
const groupIdx = ref(0)
const flowStep = ref(1)
const recCount = ref(0)
const quizWord = ref({})
const quizOptions = ref([])
const weakWord = ref({})
const quizRight = ref(0)
const quizWrong = ref(0)

const categoryName = computed(() => ({ item: '物品', person: '人物', number: '数字', emotion: '情绪' })[category.value] || '物品')
const pos = computed(() => groupIdx.value + 1)
const total = computed(() => words.value.length)
const group = computed(() => words.value)
const currentWord = computed(() => words.value[groupIdx.value])

onMounted(async () => {
  await store.bootstrap()
  unlock()
  try {
    const res = await api.words(`?category=${category.value}&child_id=${store.childId}`)
    words.value = res.words
  } catch {
    words.value = []
  }
})

function playCurrent() {
  if (!currentWord.value) return
  flowStep.value = 2
  playUrl(api.ttsUrl(currentWord.value.en, store.settings.ttsRate, store.settings.ttsVoice), { rate: store.settings.ttsRate })
}

async function nextWord() {
  // 记录 learn
  const w = currentWord.value
  if (w) {
    try {
      await api.recordLearning({ child_id: store.childId, target_type: 'word', target_id: w.id, action: 'learn' })
    } catch { /* ignore */ }
  }
  if (groupIdx.value + 1 < words.value.length) {
    groupIdx.value += 1
    flowStep.value = 1
  } else {
    // 组末：B 段小测，A 段直接鼓励（4.2）
    if (store.isBandB) {
      startQuiz()
    } else {
      view.value = 'cheer'
    }
  }
}

function prev() {
  if (groupIdx.value > 0) {
    groupIdx.value -= 1
    flowStep.value = 1
  }
}

function startQuiz() {
  const groupWords = words.value.slice(-5)
  const target = groupWords[Math.floor(Math.random() * groupWords.length)]
  const others = groupWords.filter((w) => w.id !== target.id)
  quizWord.value = target
  const wrong = others.length ? others[0] : null
  quizOptions.value = wrong ? [target, wrong].sort(() => Math.random() - 0.5) : [target]
  view.value = 'quiz'
}

function playQuiz() {
  playUrl(api.ttsUrl(quizWord.value.en, store.settings.ttsRate, store.settings.ttsVoice), { rate: store.settings.ttsRate })
}

async function answerQuiz(correct) {
  if (correct) quizRight.value += 1
  else quizWrong.value += 1
  // 记录 quiz 结果（无母亲标记时作为掌握度备用信号）
  try {
    await api.recordLearning({
      child_id: store.childId,
      target_type: 'word',
      target_id: quizWord.value.id,
      action: 'quiz',
      quiz_result: correct ? 'correct' : 'wrong',
    })
  } catch { /* ignore */ }
  // 弱词 = 本轮答错的词，否则取小测词
  weakWord.value = correct ? quizWord.value : quizWord.value
  view.value = 'summaryB'
}

function nextGroup() {
  view.value = 'learn'
  groupIdx.value = 0
  flowStep.value = 1
}
</script>
