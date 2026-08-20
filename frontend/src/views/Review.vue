<template>
  <div class="page">
    <header class="appbar">
      <router-link class="icon-btn" to="/learn" aria-label="退出复习">✕</router-link>
      <h1 style="font-size:17px">复习 · {{ idx + 1 }} / {{ queue.length }}</h1>
      <span class="icon-btn" aria-hidden="true"></span>
    </header>

    <!-- 空队列 -->
    <section v-if="queue.length === 0" class="page-body pad center stack-6">
      <div style="text-align:center">
        <div style="font-size:72px">🎉</div>
        <h2 class="t-zh-lg" style="margin:var(--sp-4) 0 var(--sp-2)">今天没有要复习的词</h2>
        <p class="t-mom">学过的词都还很稳，去学点新的吧。</p>
      </div>
      <router-link class="btn btn-primary btn-block btn-lg" to="/home">回首页</router-link>
    </section>

    <!-- ============ B 段：听音选图 ============ -->
    <section v-else-if="view === 'quizB' && current" class="page-body pad stack-6">
      <h2 class="t-zh-lg center-text">哪个是 <span style="font-size:34px">{{ current.en }}</span>？</h2>
      <div style="display:flex;justify-content:center">
        <button class="btn btn-primary btn-lg" style="min-width:190px" @click="playWord">🔊 听一听</button>
      </div>
      <div class="quiz-opts">
        <button v-for="opt in quizOptions" :key="opt.id" class="quiz-opt" @click="answerQuiz(opt.id === current.id)">
          <span class="photo"><span class="emoji">{{ opt.image_emoji || '🖼' }}</span></span>
        </button>
      </div>
      <button class="btn-quiet btn-block" @click="skipQuiz">跳过这题 ›</button>
      <p class="note">听音选图只在 B 段启用，且只做二选一（PRD 4.2 / 12.2）。拖图配对同理，对 24 月以下不可行。</p>
    </section>

    <!-- ============ A 段：只跟读，不测 ============ -->
    <section v-else-if="view === 'speakA' && current" class="page-body pad stack-6">
      <div class="photo" style="width:180px;margin:0 auto"><span class="emoji">{{ current.image_emoji || '🖼' }}</span></div>
      <div style="text-align:center">
        <div class="t-word-en">{{ current.en }}</div>
        <div class="t-phonetic">{{ current.phonetic || '' }}</div>
        <div class="t-zh" style="color:var(--c-ink-2);margin-top:var(--sp-2)">{{ current.zh }}</div>
      </div>
      <div class="row" style="gap:var(--sp-3)">
        <button class="btn btn-ghost" style="flex:none;width:76px;min-height:60px" @click="playWord">▶</button>
        <router-link class="btn btn-primary grow" style="min-height:60px" :to="compareTo">
          🎤 让宝宝跟读
        </router-link>
      </div>
      <button class="btn-quiet btn-block" @click="skip">跳过这个 ›</button>
      <p class="note">A 段复习只做「听 + 跟读 + 母亲标记」，不出任何题型。</p>
    </section>

    <!-- ============ 跟读环节（两段共用） ============ -->
    <section v-else-if="view === 'speak' && current" class="page-body pad stack-6">
      <div style="text-align:center"><span class="chip ok">✓ 选对了</span></div>
      <div class="photo" style="width:170px;margin:0 auto"><span class="emoji">{{ current.image_emoji || '🖼' }}</span></div>
      <div style="text-align:center">
        <div class="t-word-en">{{ current.en }}</div>
        <div class="t-phonetic">{{ current.phonetic || '' }}</div>
      </div>
      <div class="row" style="gap:var(--sp-3)">
        <button class="btn btn-ghost" style="flex:none;width:76px;min-height:60px" @click="playWord">▶</button>
        <router-link class="btn btn-primary grow" style="min-height:60px" :to="compareTo">🎤 再读一次</router-link>
      </div>
      <button class="btn-quiet btn-block" @click="next">下一个 ›</button>
    </section>

    <!-- ============ 复习总结 ============ -->
    <section v-else-if="view === 'done'" class="page-body pad stack-5 enter">
      <div style="text-align:center">
        <div style="font-size:80px">☑️</div>
        <h2 class="t-zh-lg" style="margin:var(--sp-4) 0 var(--sp-2)">今天的复习做完了</h2>
        <p class="t-mom">{{ queue.length }} 个词，全都跟读过一遍。</p>
      </div>

      <div class="card stack-3">
        <div class="t-label">接下来怎么排</div>
        <div class="queue">
          <div v-for="w in queue" :key="w.target_id" class="queue-row">
            <span class="photo sm"><span class="emoji">{{ w.image_emoji || '🖼' }}</span></span>
            <span class="grow">
              <div class="t-word-en-s" style="font-size:22px">{{ w.en }}</div>
              <div class="t-mom-sm">{{ w.review_label }}</div>
            </span>
            <span class="bar"><i :style="{ width: Math.round(w.mastery * 100) + '%' }"></i></span>
          </div>
        </div>
      </div>

      <router-link class="btn btn-primary btn-block btn-lg" to="/home">回首页</router-link>

      <p class="note">
        PRD 8.6：排期按掌握度分档——低于 0.3 次日再推，0.3~0.6 隔 3 天，0.6~0.85 隔 7 天，0.85 以上隔 21 天且不再主动推送。
        <b>掌握度数值和进度条只给母亲看，永不向幼儿侧暴露</b>。这里刻意用「明天 / 3 天后」这种说法，而不是「掌握度 0.24」。
      </p>
    </section>
  </div>
</template>

<script setup>
import { computed, onMounted, ref } from 'vue'
import { useAppStore } from '../stores/app'
import { api } from '../api'
import { useAudio } from '../composables/useAudio'

const store = useAppStore()
const { playUrl, unlock } = useAudio()

const queue = ref([])
const idx = ref(0)
const view = ref('quizB')
const quizOptions = ref([])

const current = computed(() => queue.value[idx.value])
const compareTo = computed(() =>
  current.value
    ? { path: '/compare', query: { target_type: 'word', target_id: current.value.target_id, en: current.value.en, zh: current.value.zh, phonetic: current.value.phonetic, emoji: current.value.image_emoji } }
    : '/home',
)

onMounted(async () => {
  await store.bootstrap()
  unlock()
  try {
    const res = await api.reviewQueue(store.childId)
    queue.value = res.queue
  } catch {
    queue.value = []
  }
  // A 段复习不测，直接跟读（4.2）
  if (store.isBandA) view.value = queue.value.length ? 'speakA' : 'done'
  else if (queue.value.length) buildQuiz()
})

function buildQuiz() {
  const target = current.value
  const distractor = queue.value.find((w) => w.target_id !== target.target_id)
  quizOptions.value = distractor ? [target, distractor].sort(() => Math.random() - 0.5) : [target]
  view.value = 'quizB'
}

function playWord() {
  if (current.value) playUrl(api.ttsUrl(current.value.en, store.settings.ttsRate, store.settings.ttsVoice), { rate: store.settings.ttsRate })
}

async function answerQuiz(correct) {
  try {
    await api.recordLearning({
      child_id: store.childId,
      target_type: 'word',
      target_id: current.value.target_id,
      action: 'quiz',
      quiz_result: correct ? 'correct' : 'wrong',
    })
  } catch { /* ignore */ }
  if (correct) view.value = 'speak'
  else next() // 答错不惩罚（12.3：舍弃 Hearts 心系统），直接下一词
}

function skipQuiz() {
  next()
}

async function skip() {
  // 记录 review（不算对错）
  if (current.value) {
    try {
      await api.recordLearning({ child_id: store.childId, target_type: 'word', target_id: current.value.target_id, action: 'review' })
    } catch { /* ignore */ }
  }
  next()
}

async function next() {
  try {
    await api.recordLearning({ child_id: store.childId, target_type: 'word', target_id: current.value.target_id, action: 'review' })
  } catch { /* ignore */ }
  if (idx.value + 1 < queue.value.length) {
    idx.value += 1
    if (store.isBandA) view.value = 'speakA'
    else buildQuiz()
  } else {
    view.value = 'done'
  }
}
</script>
