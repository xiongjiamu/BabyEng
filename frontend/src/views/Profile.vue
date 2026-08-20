<template>
  <div class="page">
    <header class="appbar">
      <span class="icon-btn" aria-hidden="true"></span>
      <h1>我的</h1>
      <span class="icon-btn" aria-hidden="true"></span>
    </header>

    <div style="padding:0 var(--sp-5) var(--sp-4)">
      <div class="seg">
        <button :aria-pressed="tab === 'daily'" @click="tab = 'daily'">日报</button>
        <button :aria-pressed="tab === 'rec'" @click="tab = 'rec'">录音</button>
        <button :aria-pressed="tab === 'medal'" @click="tab = 'medal'">成就</button>
      </div>
    </div>

    <!-- ============ 日报（7.3：轻量复盘，一屏读完） ============ -->
    <div v-if="tab === 'daily'" class="page-body pad stack-5" style="padding-top:0">
      <div class="card stack-4">
        <div class="row-between">
          <span class="t-zh" style="font-size:24px">{{ dateLabel }}</span>
          <span class="chip kid">🔥 连续 {{ report?.streak || 0 }} 天</span>
        </div>
        <div class="row" style="gap:var(--sp-6)">
          <span><div style="font-size:32px;font-weight:800">{{ report?.learned_today || 0 }}</div><div class="t-mom-sm">学了几个词</div></span>
          <span><div style="font-size:32px;font-weight:800">{{ report?.rec_today || 0 }}</div><div class="t-mom-sm">跟读次数</div></span>
          <span><div style="font-size:32px;font-weight:800">{{ report?.parent_time_min || 0 }}′</div><div class="t-mom-sm">亲子时长</div></span>
        </div>
        <div class="divider"></div>
        <div class="stack-3">
          <div class="t-label">今天新学</div>
          <div class="newword">
            <span v-for="w in report?.new_words || []" :key="w.id">{{ w.en }}</span>
            <span v-if="!report?.new_words?.length" style="background:var(--c-surface-2);color:var(--c-ink-3)">今天还没学</span>
          </div>
        </div>
      </div>

      <div class="card stack-3">
        <div class="t-label">妈妈学习卡</div>
        <div v-for="w in (report?.new_words || []).slice(0, 1)" :key="w.id" class="row" style="align-items:flex-start;gap:var(--sp-3)">
          <span style="font-size:24px">📖</span>
          <div class="grow">
            <div class="t-word-en-s" style="font-size:24px">{{ w.en }} {{ w.phonetic || '' }}</div>
            <p class="t-mom" style="margin:6px 0 0">发音要点见词条页的「妈妈学习卡」。</p>
          </div>
        </div>
        <p v-if="!(report?.new_words || []).length" class="t-mom-sm" style="margin:0">学一个新词，这里就会出现给你的发音要点。</p>
      </div>

      <div class="card stack-3">
        <div class="t-label">明天先复习</div>
        <div class="newword">
          <span v-for="w in report?.tomorrow_review || []" :key="w.id">{{ w.en }}</span>
          <span v-if="!(report?.tomorrow_review || []).length" style="background:var(--c-surface-2);color:var(--c-ink-3)">暂无</span>
        </div>
        <p class="t-mom-sm" style="margin:0">这些词的掌握度还不稳，明天优先复习。</p>
      </div>

      <div class="banner info">
        <span class="ico">🎧</span>
        <span>今天亲子时长 {{ report?.parent_time_min || 0 }} 分钟，<b>宝宝屏幕时间 {{ report?.screen_sec_today || 0 }} 秒</b>。</span>
      </div>

      <p class="note">日报用于快速回顾当天的学习情况，每日 21:00 生成。</p>
    </div>

    <!-- ============ 录音（8.4 / 11.4：30 天保留 + 收藏） ============ -->
    <div v-else-if="tab === 'rec'" class="page-body pad stack-5" style="padding-top:0">
      <div class="banner info">
        <span class="ico">🗓</span>
        <span>录音默认保留 30 天。点右侧 ♡ 收藏后长期保留，不受清理影响。</span>
      </div>
      <div class="list">
        <div v-for="r in recs" :key="r.id" class="rec-row">
          <button class="play" @click="playRec(r)">▶</button>
          <span class="grow">
            <div class="t-word-en-s" style="font-size:22px">{{ r.en || '未知' }}</div>
            <div class="t-mom-sm">{{ fmtTime(r.created_at) }} · {{ fmtDur(r.duration_ms) }}</div>
          </span>
          <button class="icon-btn" style="width:40px;height:40px;color:var(--c-primary-deep)" @click="toggleFav(r)">
            {{ r.favorited ? '♥' : '♡' }}
          </button>
        </div>
        <div v-if="recs.length === 0" class="list-item" style="justify-content:center;color:var(--c-ink-3)">还没有录音，去让宝宝跟读一次吧</div>
      </div>
      <p class="note">录音全部存在自己的服务器上，30 天自动过期，设置页可一键清理或导出。</p>
    </div>

    <!-- ============ 成就（7.1：打卡日历 + 勋章墙，无 XP/排行榜） ============ -->
    <div v-else class="page-body pad stack-5" style="padding-top:0">
      <div class="card stack-4">
        <div class="row-between">
          <span class="t-label">{{ monthLabel }} 打卡</span>
          <span class="chip mom">❄ 保护还剩 {{ cal?.freeze_left || 0 }} 次</span>
        </div>
        <div class="cal">
          <span v-for="(d, i) in calHeader" :key="'h' + i" class="hd">{{ d }}</span>
          <template v-for="(cell, i) in calCells" :key="i">
            <span v-if="cell.blank"></span>
            <span v-else :class="{ on: cell.on, freeze: cell.frozen }">{{ cell.day }}</span>
          </template>
        </div>
        <p class="t-mom-sm" style="margin:0">❄ 用了打卡保护的当天，连续天数不断。每月自动送 2 次。</p>
      </div>

      <div class="stack-3">
        <div class="t-label">勋章</div>
        <div class="medals">
          <div v-for="m in medals" :key="m.key" class="medal" :class="{ locked: !m.unlocked }">
            <span class="ico">{{ m.icon }}</span>
            <span class="t">{{ m.name }}</span>
          </div>
        </div>
      </div>

      <p class="note">
        没有 XP 数字、排行榜或心数惩罚。连续打卡天数<b>不作为学习效果指标</b>——
        它只是降低放弃成本的脚手架，一旦当成目标就会诱发「为保打卡而打卡」。
      </p>
    </div>

    <TabBar current="profile" />
  </div>
</template>

<script setup>
import { computed, onMounted, ref } from 'vue'
import TabBar from '../components/TabBar.vue'
import { useAppStore } from '../stores/app'
import { api } from '../api'
import { useAudio } from '../composables/useAudio'

const store = useAppStore()
const { playUrl, unlock } = useAudio()

const tab = ref('daily')
const report = ref(null)
const recs = ref([])
const cal = ref(null)
const medals = ref([])

const dateLabel = computed(() => {
  const d = new Date()
  return `${d.getMonth() + 1} 月 ${d.getDate()} 日`
})
const monthLabel = computed(() => {
  const d = new Date()
  return `${d.getFullYear()} 年 ${d.getMonth() + 1} 月`
})
const calHeader = ['日', '一', '二', '三', '四', '五', '六']
const calCells = computed(() => {
  const now = new Date()
  const year = now.getFullYear()
  const month = now.getMonth()
  const first = new Date(year, month, 1)
  const daysInMonth = new Date(year, month + 1, 0).getDate()
  const map = {}
  ;(cal.value?.calendar?.days || []).forEach((d) => {
    map[d.day] = d.frozen ? 'frozen' : 'on'
  })
  const cells = []
  for (let i = 0; i < first.getDay(); i++) cells.push({ blank: true })
  for (let d = 1; d <= daysInMonth; d++) {
    const key = `${year}-${String(month + 1).padStart(2, '0')}-${String(d).padStart(2, '0')}`
    const v = map[key]
    cells.push({ day: d, on: v === 'on', frozen: v === 'frozen' })
  }
  return cells
})

onMounted(async () => {
  await store.bootstrap()
  unlock()
  if (!store.initialized) { location.href = '/onboarding'; return }
  const cid = store.childId
  try {
    const [r, rec, c, m] = await Promise.all([
      api.reportToday(cid),
      api.recordingsToday(cid),
      api.reportCalendar(cid),
      api.achievements(cid),
    ])
    report.value = r
    recs.value = rec.recordings
    cal.value = c
    medals.value = m.medals
  } catch { /* 离线默认 */ }
})

function fmtTime(iso) {
  if (!iso) return ''
  const d = new Date(iso)
  const now = new Date()
  const sameDay = d.toDateString() === now.toDateString()
  if (sameDay) return `今天 ${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`
  return `${d.getMonth() + 1} 月 ${d.getDate()} 日`
}
function fmtDur(ms) {
  return `0:${String(Math.floor((ms || 0) / 1000)).padStart(2, '0')}`
}
function playRec(r) {
  playUrl(api.recordingUrl(r.id))
}
async function toggleFav(r) {
  const next = !r.favorited
  try {
    await api.favoriteRecording(r.id, next)
    r.favorited = next
  } catch { /* ignore */ }
}
</script>
