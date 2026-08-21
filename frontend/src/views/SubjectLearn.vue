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
      <div class="subject-emoji">{{ current.image_emoji }}</div>
      <div class="center-text">
        <h2 class="subject-title">{{ current.title }}</h2>
        <p class="t-zh-lg">{{ current.prompt }}</p>
      </div>
      <div class="card stack-3 subject-guide">
        <div class="t-label">放下手机一起做</div>
        <div><b>准备：</b>{{ activityGuide.materials }}</div>
        <div><b>妈妈说：</b>“{{ activityGuide.parentScript }}”</div>
        <div><b>宝宝做：</b>{{ activityGuide.childAction }}</div>
      </div>
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

const activityGuide = computed(() => {
  const guides = {
    character: { materials: '家里的同类实物或宝宝自己的身体', parentScript: `找找和“${current.value?.title || ''}”有关的东西。`, actionA: '跟着妈妈看一看、摸一摸。', actionB: '自己指出或拿出对应的东西。' },
    opposite: { materials: '两个大小、多少或位置不同的实物', parentScript: current.value?.prompt || '我们来比一比。', actionA: '跟着妈妈分别摸一摸两个物品。', actionB: '按妈妈的要求指出其中一个。' },
    rhyme: { materials: '不需要材料，留出一点活动空间', parentScript: current.value?.prompt || '我们一起念一念。', actionA: '听妈妈念，跟着拍手或做动作。', actionB: '跟念一个词或模仿一个动作。' },
    counting: { materials: '同类安全实物 1～4 个，例如积木或袜子', parentScript: current.value?.prompt || '我们一个一个数。', actionA: '看妈妈逐个触碰物品并数数。', actionB: '自己逐个移动物品，妈妈同步数数。' },
    quantity: { materials: '数量明显不同的两小堆安全实物', parentScript: current.value?.prompt || '哪边多，哪边少？', actionA: '看妈妈把两堆排开并比较。', actionB: '指出多的一边，答错也不纠缠。' },
    shape: { materials: '家里相似形状的安全物品', parentScript: `我们在家里找一个${current.value?.answer || '这样的形状'}。`, actionA: '跟妈妈摸一摸物品的边缘。', actionB: '从两个物品中找出相同形状。' },
  }
  const guide = guides[current.value?.category] || { materials: '一个家里的安全实物', parentScript: current.value?.prompt || '我们一起试试。', actionA: '跟着妈妈做动作。', actionB: '自己试着完成动作。' }
  return { materials: guide.materials, parentScript: guide.parentScript, childAction: store.isBandA ? guide.actionA : guide.actionB }
})

onMounted(async () => {
  await store.bootstrap()
  await loadItems()
})

async function loadItems() {
  loadError.value = false
  try {
    const result = await api.subjectItems(subject.value, store.childId)
    const all = result.items || []
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
.subject-title { margin: 0; font-size: 34px; }
.subject-guide { width: 100%; font-size: var(--fs-mom); line-height: 1.55; }
</style>
