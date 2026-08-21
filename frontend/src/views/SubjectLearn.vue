<template>
  <div class="page">
    <header class="appbar">
      <router-link class="icon-btn" to="/learn" aria-label="退出学习">✕</router-link>
      <h1>{{ subjectLabel }} · 第 {{ index + 1 }} / {{ items.length }} 个活动</h1>
      <span class="icon-btn" aria-hidden="true"></span>
    </header>
    <div class="steps"><i v-for="(item, i) in items" :key="item.id" :class="{ done: i < index, now: i === index }"></i></div>

    <section v-if="loadError" class="page-body pad center stack-6">
      <div class="subject-emoji">📚</div>
      <h2 class="t-zh-lg">内容暂时没有加载出来</h2>
      <p class="t-mom">请检查网络后再试一次。</p>
      <button class="btn btn-primary btn-block" @click="loadItems">重新加载</button>
    </section>

    <section v-else-if="view === 'learn' && current" class="page-body pad center stack-5">
      <div v-if="saveError" class="banner warn"><span class="ico">⚠️</span><span>刚才的观察没有保存，活动仍可继续。</span></div>
      <span class="chip kid">{{ categoryLabel }}</span>
      <ContentImage class="subject-image" kind="activity" :target-id="current.id" :emoji="current.image_emoji" :alt="current.title" />
      <div class="center-text">
        <h2 class="subject-title">{{ current.title }}</h2>
        <p class="t-zh-lg">{{ current.prompt }}</p>
      </div>
      <div class="card stack-3 subject-guide">
        <div class="t-label">放下手机一起做</div>
        <div><b>准备：</b>{{ activityGuide.materials }}</div>
        <div><b>妈妈说：</b>“{{ activityGuide.parentScript }}”</div>
        <div><b>宝宝做：</b>{{ activityGuide.childAction }}</div>
        <div><b>观察：</b>{{ activityGuide.observeFor }}</div>
      </div>
      <div class="banner warn"><span class="ico">🛡️</span><span>{{ activityGuide.safetyNote }}</span></div>
      <div class="stack-3" style="width:100%">
        <button class="btn btn-primary btn-block" :disabled="saving" @click="complete('observed_with_help')">一起做过了</button>
        <button class="btn btn-ghost btn-block" :disabled="saving" @click="complete('observed_independent')">宝宝自己完成了</button>
        <button class="btn-quiet btn-block" :disabled="saving" @click="complete('not_interested')">今天没兴趣，先跳过</button>
      </div>
      <p class="note">这里只记录今天的表现，不给宝宝打分，也不代表已经“学会”。</p>
    </section>

    <section v-else class="page-body pad center stack-6">
      <div class="subject-emoji">🌿</div>
      <h2 class="t-zh-lg">这一轮结束啦</h2>
      <p class="t-mom">今天一起做了 {{ completedCount }} 个小活动。</p>
      <p class="t-mom center-text">不用把卡片全部做完，宝宝还愿意玩就是最好的结束时机。</p>
      <router-link class="btn btn-primary btn-block btn-lg" to="/home">回首页</router-link>
      <button class="btn-quiet btn-block" @click="restart">再做一遍</button>
    </section>
  </div>
</template>

<script setup>
import { computed, onMounted, ref } from 'vue'
import { useRoute } from 'vue-router'
import { api } from '../api'
import { useAppStore } from '../stores/app'
import ContentImage from '../components/ContentImage.vue'

const route = useRoute()
const store = useAppStore()
const subject = computed(() => route.params.subject === 'math' ? 'math' : 'chinese')
const subjectLabel = computed(() => subject.value === 'math' ? '数学亲子活动' : '语言亲子活动')
const items = ref([])
const index = ref(0)
const view = ref('learn')
const loadError = ref(false)
const saving = ref(false)
const saveError = ref(false)
const completedCount = ref(0)
const current = computed(() => items.value[index.value])
const categoryLabel = computed(() => ({ character: '看物说话', opposite: '比较概念', rhyme: '儿歌动作', counting: '实物数数', quantity: '比多少', shape: '寻找形状' })[current.value?.category] || '亲子活动')

const activityGuide = computed(() => ({
  materials: current.value?.materials || '一个家里的安全实物',
  parentScript: current.value?.parent_script || current.value?.prompt || '我们一起试试。',
  childAction: store.isBandA ? current.value?.child_action_a : current.value?.child_action_b,
  observeFor: current.value?.observe_for || '观察宝宝是否愿意共同注意或动手尝试。',
  safetyNote: current.value?.safety_note || '全程由成人陪伴。',
}))

onMounted(async () => {
  await store.bootstrap()
  await loadItems()
})

async function loadItems() {
  loadError.value = false
  try {
    const result = await api.subjectItems(subject.value, store.childId)
    const all = result.items || []
    const requested = typeof route.query.activity === 'string' ? all.find((item) => item.id === route.query.activity) : null
    if (requested) {
      items.value = [requested]
      return
    }
    const ordered = [...all.filter((item) => !item.learned), ...all.filter((item) => item.learned)]
    items.value = ordered.slice(0, 3)
    loadError.value = items.value.length === 0
  } catch { loadError.value = true }
}

async function complete(motherMark) {
  if (!current.value || saving.value) return
  saving.value = true
  saveError.value = false
  const didActivity = motherMark !== 'not_interested'
  try {
    await api.recordLearning({ child_id: store.childId, target_type: 'subject_item', target_id: current.value.id, action: didActivity ? 'learn' : 'observe', mother_mark: motherMark })
  } catch { saveError.value = true }
  if (didActivity) completedCount.value += 1
  saving.value = false
  if (index.value + 1 < items.value.length) index.value += 1
  else view.value = 'summary'
}

function restart() {
  index.value = 0
  completedCount.value = 0
  view.value = 'learn'
}
</script>

<style scoped>
.subject-emoji { font-size: 82px; line-height: 1; }
.subject-image { width:180px;height:180px;border-radius:var(--r-lg);background:var(--c-surface-2);font-size:82px; }
.subject-title { margin: 0; font-size: 34px; }
.subject-guide { width: 100%; font-size: var(--fs-mom); line-height: 1.55; }
</style>
