<template>
  <div class="page">
    <header class="appbar">
      <router-link class="icon-btn" to="/learn" aria-label="退出学习">✕</router-link>
      <h1>{{ subjectLabel }} · 第 {{ index + 1 }} / {{ items.length }} 个</h1>
      <span class="icon-btn" aria-hidden="true"></span>
    </header>

    <div class="steps"><i v-for="(item, i) in items" :key="item.id" :class="{ done: i < index, now: i === index }"></i></div>

    <section v-if="loadError" class="page-body pad center stack-6">
      <div class="subject-emoji">📚</div>
      <h2 class="t-zh-lg">内容暂时没有加载出来</h2>
      <p class="t-mom">请检查网络后再试一次。</p>
      <button class="btn btn-primary btn-block" @click="loadItems">重新加载</button>
    </section>

    <section v-else-if="view === 'learn' && current" class="page-body pad center stack-6">
      <span class="chip kid">{{ categoryLabel }}</span>
      <div class="subject-emoji">{{ current.image_emoji }}</div>
      <div class="center-text">
        <h2 class="subject-title">{{ current.title }}</h2>
        <p class="t-zh-lg">{{ current.prompt }}</p>
        <p class="subject-answer">{{ current.answer }}</p>
      </div>
      <div class="card stack-3 subject-guide">
        <div class="t-label">亲子互动</div>
        <div>妈妈先读一遍，再请宝宝指一指、说一说或动手摆一摆。</div>
      </div>
      <div class="row" style="width:100%;gap:var(--sp-3)">
        <button class="btn btn-ghost grow" :disabled="index === 0" @click="previous">‹ 上一个</button>
        <button class="btn btn-primary grow" @click="next">学会了，下一个 ›</button>
      </div>
    </section>

    <section v-else-if="view === 'quiz'" class="page-body pad center stack-6">
      <span class="chip kid">小测 · 这一组学完了</span>
      <div class="subject-emoji">{{ quizItem.image_emoji }}</div>
      <h2 class="t-zh-lg center-text">{{ quizItem.prompt }}</h2>
      <div class="quiz-opts subject-options">
        <button v-for="option in quizOptions" :key="option" class="quiz-opt subject-option" @click="answer(option)">{{ option }}</button>
      </div>
    </section>

    <section v-else class="page-body pad center stack-6">
      <div class="subject-emoji">🎉</div>
      <h2 class="t-zh-lg">这一组学完啦</h2>
      <p class="t-mom">今天认识了 {{ items.length }} 个{{ subjectLabel }}小知识。</p>
      <router-link class="btn btn-primary btn-block btn-lg" to="/learn">选择其他学习内容</router-link>
      <button class="btn-quiet btn-block" @click="restart">再学一遍</button>
    </section>
  </div>
</template>

<script setup>
import { computed, onMounted, ref } from 'vue'
import { useRoute } from 'vue-router'
import { api } from '../api'
import { useAppStore } from '../stores/app'

const route = useRoute()
const store = useAppStore()
const subject = computed(() => route.params.subject === 'math' ? 'math' : 'chinese')
const subjectLabel = computed(() => subject.value === 'math' ? '数学启蒙' : '语文启蒙')
const items = ref([])
const index = ref(0)
const view = ref('learn')
const quizItem = ref({})
const quizOptions = ref([])
const loadError = ref(false)
const current = computed(() => items.value[index.value])
const categoryLabel = computed(() => ({ character: '常用字', opposite: '反义词', rhyme: '儿歌', counting: '数数', quantity: '比多少', shape: '认识形状' })[current.value?.category] || '启蒙')

onMounted(async () => {
  await store.bootstrap()
  await loadItems()
})

async function loadItems() {
  loadError.value = false
  try {
    const result = await api.subjectItems(subject.value, store.childId)
    items.value = result.items || []
    loadError.value = items.value.length === 0
  } catch { loadError.value = true }
}

async function next() {
  if (!current.value) return
  try { await api.recordLearning({ child_id: store.childId, target_type: 'subject_item', target_id: current.value.id, action: 'learn' }) } catch { /* 离线时仍可继续学习 */ }
  if (index.value + 1 < items.value.length) { index.value += 1; return }
  if (store.isBandB && items.value.length > 1) startQuiz()
  else view.value = 'summary'
}

function previous() { if (index.value > 0) index.value -= 1 }

function startQuiz() {
  quizItem.value = items.value[Math.floor(Math.random() * items.value.length)]
  const wrong = (items.value.find((item) => item.id !== quizItem.value.id && item.category === quizItem.value.category)
    || items.value.find((item) => item.id !== quizItem.value.id))?.answer
  quizOptions.value = [quizItem.value.answer, wrong].filter(Boolean).sort(() => Math.random() - 0.5)
  view.value = 'quiz'
}

async function answer(option) {
  try { await api.recordLearning({ child_id: store.childId, target_type: 'subject_item', target_id: quizItem.value.id, action: 'quiz', quiz_result: option === quizItem.value.answer ? 'correct' : 'wrong' }) } catch { /* 离线时仍可完成 */ }
  view.value = 'summary'
}

function restart() { index.value = 0; view.value = 'learn' }
</script>

<style scoped>
.subject-emoji { font-size: 88px; line-height: 1; }
.subject-title { margin: 0; font-size: 38px; }
.subject-answer { margin: var(--sp-3) 0 0; font-size: 28px; font-weight: 800; color: var(--c-primary-strong); }
.subject-guide { width: 100%; font-size: var(--fs-mom); line-height: 1.6; }
.subject-options { width: 100%; }
.subject-option { min-height: 112px; font-size: 30px; font-weight: 800; }
</style>
