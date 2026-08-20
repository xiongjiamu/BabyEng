<template>
  <div class="page">
    <header class="appbar">
      <span class="icon-btn" aria-hidden="true"></span>
      <h1 style="font-size:18px">BabyEng</h1>
      <router-link class="icon-btn" to="/settings" aria-label="设置">⚙</router-link>
    </header>

    <!-- 推理服务未就绪提示（5.4） -->
    <div v-if="!allSvcReady && store.initialized" class="device-banner" style="padding:0 var(--sp-5)">
      <div class="banner info">
        <span class="ico">⏳</span>
        <span><b>正在启动，大约还要 1 分钟</b><br>发音和识别暂时不可用，可以先翻翻词</span>
      </div>
    </div>

    <div class="greet">
      <span class="avatar">🐙</span>
      <div class="grow">
        <div style="font-size:19px;font-weight:800">{{ greeting }}，{{ childName }}</div>
        <div class="t-mom-sm">{{ bandLabel }} · {{ audioOnlyLabel }}</div>
      </div>
      <span class="chip kid">🔥 {{ summary?.streak || 0 }} 天</span>
    </div>

    <div class="page-body pad stack-6" style="padding-top:0">
      <!-- 今日进度（5.1 顶部区） -->
      <div class="card today">
        <div class="row-between">
          <span class="t-mom" style="font-weight:700;color:var(--c-ink)">今天学了 {{ summary?.learned_today || 0 }} 个词</span>
          <span class="t-mom-sm">目标 {{ summary?.daily_goal || 5 }} 个</span>
        </div>
        <div class="progress"><i :style="{ width: todayProgress + '%' }"></i></div>
        <div class="row-between">
          <span class="streak" :aria-label="'连续打卡 ' + (summary?.streak || 0) + ' 天'">
            <template v-for="(d, i) in week" :key="i">
              <i :class="{ on: d.on, freeze: d.freeze }">{{ d.label }}</i>
            </template>
          </span>
          <router-link class="t-mom-sm" to="/profile" style="text-decoration:none;font-weight:700;color:var(--c-mom)">日报 →</router-link>
        </div>
        <p v-if="summary && summary.freeze_left > 0" class="t-mom-sm" style="margin:0">❄ 是打卡保护，本月还剩 {{ summary.freeze_left }} 次</p>
      </div>

      <!-- 主入口（5.1 三大按钮：MVP 只有问一问可用） -->
      <router-link class="hero" to="/ask">
        <span class="ico">🎙</span>
        <span class="grow">
          <span class="t">问一问</span>
          <span class="s">看到什么就问什么，马上教</span>
        </span>
        <span style="font-size:28px">›</span>
      </router-link>

      <div class="duo">
        <router-link class="tile" to="/learn">
          <span class="ico">📚</span>
          <span class="t">学单词</span>
          <span class="s">按场景成组学</span>
        </router-link>
        <router-link class="tile" to="/review">
          <span class="ico">🔁</span>
          <span class="t">复习</span>
          <span class="s">{{ reviewCount }} 个待复习</span>
        </router-link>
      </div>

      <!-- 纯音频模式入口（A 段默认，6.6） -->
      <router-link class="card row" to="/audio" style="text-decoration:none;color:inherit;gap:var(--sp-4)">
        <span style="font-size:30px">🎧</span>
        <span class="grow">
          <div style="font-size:19px;font-weight:800">纯音频模式</div>
          <div class="t-mom-sm">宝宝不看屏幕，只听声音 · 不计入屏幕时间</div>
        </span>
        <span style="font-size:24px;color:var(--c-ink-3)">›</span>
      </router-link>

      <!-- 场景快捷区（5.1 六类） -->
      <div class="stack-3">
        <div class="t-label">按场景找</div>
        <div class="scenes">
          <router-link class="scene" :to="{ path: '/word-learn', query: { category: 'item' } }"><span class="ico">🧸</span><span class="t">物品</span></router-link>
          <router-link class="scene" :to="{ path: '/word-learn', query: { category: 'person' } }"><span class="ico">👪</span><span class="t">人物</span></router-link>
          <router-link class="scene" :to="{ path: '/word-learn', query: { category: 'number' } }"><span class="ico">🔢</span><span class="t">数字</span></router-link>
          <router-link class="scene" :to="{ path: '/word-learn', query: { category: 'emotion' } }"><span class="ico">😊</span><span class="t">情绪</span></router-link>
          <router-link class="scene" to="/sentences"><span class="ico">💬</span><span class="t">短句</span></router-link>
          <router-link class="scene" to="/profile"><span class="ico">📖</span><span class="t">妈妈卡</span></router-link>
        </div>
      </div>

      <p class="note">
        语言启蒙靠每天一点：问 3~5 个词、跟读一遍，比一次学一堆管用。<br>
        本应用替代不了真人互动，母亲全程参与是它成立的前提。
      </p>
    </div>

    <TabBar current="home" />
  </div>
</template>

<script setup>
import { computed, onMounted, ref } from 'vue'
import TabBar from '../components/TabBar.vue'
import { useAppStore } from '../stores/app'
import { api } from '../api'

const store = useAppStore()
const summary = ref(null)
const reviewCount = ref(0)

const greeting = computed(() => {
  const h = new Date().getHours()
  if (h < 11) return '早上好'
  if (h < 14) return '中午好'
  if (h < 18) return '下午好'
  return '晚上好'
})
const childName = computed(() => store.child?.child_name || '宝宝')
const bandLabel = computed(() => {
  if (!store.ageBand) return '宝宝档案待补'
  return `${store.ageBand} 段${store.ageMonths ? ' · ' + store.ageMonths + ' 个月' : ''}`
})
const audioOnlyLabel = computed(() => (store.settings.audioOnly ? '纯音频模式已开启' : '看图模式'))
const todayProgress = computed(() => {
  const s = summary.value
  if (!s) return 0
  return Math.min(100, Math.round((s.learned_today / Math.max(s.daily_goal, 1)) * 100))
})
const allSvcReady = computed(() => store.svcReady.tts && store.svcReady.asr)
const week = computed(() => {
  // 最近 7 天打卡（周五六日或一二三…：与原型一致的周视图，这里展示最近 7 天）
  const labels = ['日', '一', '二', '三', '四', '五', '六']
  const out = []
  const today = new Date()
  for (let i = 6; i >= 0; i--) {
    const d = new Date(today)
    d.setDate(today.getDate() - i)
    const key = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
    out.push({ label: labels[d.getDay()], on: !!checkins.value[key], freeze: !!checkins.value[key + '_f'] })
  }
  return out
})
const checkins = ref({})

onMounted(async () => {
  await store.bootstrap()
  // 未初始化 → 引导页
  if (!store.initialized) {
    location.href = '/onboarding'
    return
  }
  try {
    const cid = store.childId
    const [s, q] = await Promise.all([
      api.progressSummary(cid),
      api.reviewQueue(cid),
    ])
    summary.value = s
    reviewCount.value = q.count
    // 打卡日历（本周高亮）
    const cal = await api.reportCalendar(cid)
    const map = {}
    ;(cal.calendar?.days || []).forEach((d) => {
      map[d.day] = true
      if (d.frozen) map[d.day + '_f'] = true
    })
    checkins.value = map
  } catch {
    /* 离线模式保持默认值 */
  }
  store.refreshSvcStatus()
})
</script>
