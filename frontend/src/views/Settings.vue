<template>
  <div class="page">
    <header class="appbar">
      <router-link class="icon-btn" to="/home" aria-label="返回">←</router-link>
      <h1>设置</h1>
      <span class="icon-btn" aria-hidden="true"></span>
    </header>

    <div class="page-body pad stack-6">
      <!-- 宝宝 -->
      <div class="group">
        <div class="t-label">宝宝</div>
        <div class="list">
          <button class="list-item" @click="openNameEditor"><span class="grow">名字</span><span class="val">{{ store.child?.child_name || '未填' }} ›</span></button>
          <div class="list-item"><span class="grow">出生年月</span><span class="val">{{ store.child?.child_birthdate || '待补' }} ›</span></div>
          <div class="list-item">
            <span class="grow">
              年龄分段
              <div class="sub">{{ store.ageMonths ? `按 ${store.ageMonths} 月龄` : '未填生日' }}{{ store.ageBand ? `判定为 ${store.ageBand} 段` : '' }}</div>
            </span>
            <span class="seg" style="width:132px">
              <button :aria-pressed="store.ageBand === 'A'" @click="setBand('A')">A 段</button>
              <button :aria-pressed="store.ageBand === 'B'" @click="setBand('B')">B 段</button>
            </span>
          </div>
          <div class="list-item">
            <span class="grow">难度</span>
            <span class="seg" style="width:180px">
              <button v-for="lv in [1, 2, 3]" :key="lv" :aria-pressed="store.child?.level === lv" @click="setLevel(lv)">L{{ lv }}</button>
            </span>
          </div>
        </div>
      </div>

      <!-- 学习与屏幕时间（11.3） -->
      <div class="group">
        <div class="t-label">学习与屏幕时间</div>
        <div class="list">
          <div class="list-item">
            <span class="grow">
              纯音频模式
              <div class="sub">宝宝只听声音，不看屏幕</div>
            </span>
            <button class="switch" role="switch" :aria-checked="store.settings.audioOnly" @click="toggleAudioOnly" aria-label="纯音频模式"></button>
          </div>
          <div class="slider-row">
            <div class="row-between"><span>发音语速</span><span class="val">{{ (store.settings.ttsRate * 1).toFixed(1) }}×</span></div>
            <input type="range" min="60" max="110" :value="store.settings.ttsRate * 100" step="10" @input="onRate" />
            <div class="ticks"><span>0.6× 更慢</span><span>1.0× 正常</span></div>
          </div>
          <div class="list-item">
            <span class="grow">单次会话时长</span>
            <span class="val">{{ store.settings.sessionLimitMin }} 分钟 ›</span>
          </div>
          <div class="list-item">
            <span class="grow">
              每日屏幕时间上限
              <div class="sub">纯音频模式的时长不计入这里</div>
            </span>
            <span class="val">{{ store.settings.screenLimitMin }} 分钟 <span v-if="store.isBandA" class="cap">仅可下调</span></span>
          </div>
          <div class="list-item">
            <span class="grow">
              睡前提醒
              <div class="sub">到点柔性收尾，不硬中断</div>
            </span>
            <span class="val">{{ store.settings.bedtimeHour }}:00 ›</span>
          </div>
        </div>
        <p class="note">
          A 段（12~24 月）默认使用纯音频模式；B 段默认屏幕时间上限为 15 分钟。
          上限<b>只能下调不能上调</b>——主流育儿指南对 24 月龄以下建议不安排屏幕时间。
        </p>
      </div>

      <!-- 发音与模型 -->
      <div class="group">
        <div class="t-label">发音与模型</div>
        <div class="list">
          <button class="list-item" @click="voiceSheet = true">
            <span class="grow">
              英语音色
              <div class="sub">美式 · Piper {{ currentVoiceLabel }}</div>
            </span>
            <span class="val">›</span>
          </button>
          <div class="list-item">
            <span class="grow">
              使用云端模型
              <div class="sub">词库查不到的词，交给云端 AI 生成</div>
            </span>
            <button class="switch" role="switch" :aria-checked="store.settings.cloudModel" @click="askCloud" aria-label="使用云端模型"></button>
          </div>
        </div>
        <p class="note">学习数据默认只在本地处理。主动开启云端模型前，会说明数据用途并再次确认。</p>
      </div>

      <!-- 数据 -->
      <div class="group">
        <div class="t-label">数据</div>
        <div class="list">
          <div class="list-item">
            <span class="grow">
              录音保留
              <div class="sub">收藏过的不受影响</div>
            </span>
            <span class="val">30 天 ›</span>
          </div>
          <button class="list-item" @click="cleanup"><span class="grow">清理 30 天前的录音</span><span class="val">›</span></button>
          <button class="list-item" @click="exportData"><span class="grow">导出全部数据</span><span class="val">›</span></button>
          <button class="list-item danger" @click="confirmClear"><span class="grow">清空所有录音与记录</span><span class="val">›</span></button>
        </div>
      </div>

      <!-- 关于 -->
      <div class="group">
        <div class="t-label">关于</div>
        <div class="list">
          <div class="list-item"><span class="grow">当前账号</span><span class="val">{{ username }}</span></div>
          <button class="list-item danger" @click="logout"><span class="grow">退出登录</span><span class="val">›</span></button>
          <div class="list-item"><span class="grow">版本</span><span class="val">MVP 0.4</span></div>
        </div>
      </div>
    </div>

    <TabBar current="settings" />

    <div v-if="nameSheet" class="overlay">
      <div class="sheet stack-5">
        <h2 class="t-zh-lg">修改宝宝姓名</h2>
        <div class="textinput">
          <input v-model="childNameDraft" maxlength="20" type="text" placeholder="请输入宝宝姓名" @keyup.enter="saveChildName" />
        </div>
        <p v-if="nameError" class="note" style="color:var(--c-danger)">{{ nameError }}</p>
        <button class="btn btn-primary btn-block btn-lg" :disabled="nameSaving" @click="saveChildName">{{ nameSaving ? '保存中…' : '保存' }}</button>
        <button class="btn btn-ghost btn-block" :disabled="nameSaving" @click="nameSheet = false">取消</button>
      </div>
    </div>

    <div v-if="voiceSheet" class="overlay">
      <div class="sheet stack-5">
        <h2 class="t-zh-lg">选择英语音色</h2>
        <div class="list">
          <button v-for="voice in voices" :key="voice.id" class="list-item" @click="selectVoice(voice.id)">
            <span class="grow"><b>{{ voice.label }}</b><span class="sub">{{ voice.description }}</span></span>
            <span class="val">{{ store.settings.ttsVoice === voice.id ? '✓' : '›' }}</span>
          </button>
        </div>
        <button class="btn btn-mom btn-block" @click="previewVoice">试听当前音色</button>
        <button class="btn btn-ghost btn-block" @click="voiceSheet = false">完成</button>
      </div>
    </div>

    <!-- 云端模型知情同意（11.4 强制要求） -->
    <div v-if="cloudSheet" class="overlay">
      <div class="sheet stack-5">
        <h2 class="t-zh-lg">开启前，先说清楚会发生什么</h2>

        <div class="stack-3">
          <div class="banner ok">
            <span class="ico">✅</span>
            <span><b>只会发出去这一样</b><br>你提问的中文文字，比如「杯子」</span>
          </div>
          <div class="banner danger">
            <span class="ico">🚫</span>
            <span><b>这些永远不会离开你的服务器</b><br>录音原声、宝宝的名字和生日、全部学习记录与跟读录音</span>
          </div>
        </div>

        <p class="t-mom" style="margin:0">
          提问文字里可能带上宝宝的小名或家里的称呼，请你自己判断是否接受。
          关掉之后立刻恢复全本地，已发出的内容由对方服务商的政策约束，我们无法追回。
        </p>

        <div class="stack-3">
          <button class="btn btn-mom btn-block btn-lg" @click="confirmCloud(true)">我知道了，开启</button>
          <button class="btn btn-ghost btn-block" @click="confirmCloud(false)">先不开</button>
        </div>

        <p class="note" style="margin:0">
          确认状态入库且可随时撤销。产品对外的隐私说明措辞为「默认全本地存储；云端模型为可选项，开启后仅外发提问文本」。
        </p>
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'
import TabBar from '../components/TabBar.vue'
import { useAppStore } from '../stores/app'
import { api } from '../api'
import { useAudio } from '../composables/useAudio'

const store = useAppStore()
const router = useRouter()
const cloudSheet = ref(false)
const nameSheet = ref(false)
const voiceSheet = ref(false)
const childNameDraft = ref('')
const nameError = ref('')
const nameSaving = ref(false)
const username = localStorage.getItem('babyeng_username') || ''
const { unlock, playUrl } = useAudio()
const voices = [
  { id: 'en_US-mike-medium', label: 'Mike', description: '英语（美国）' },
  { id: 'en_US-amy-medium', label: 'Amy', description: '英语（美国）' },
  { id: 'en_US-ryan-medium', label: 'Ryan', description: '英语（美国）' },
  { id: 'en_US-kristin-medium', label: 'Kristin', description: '英语（美国）' },
  { id: 'en_US-hfc_female-medium', label: 'HFC Female', description: '英语（美国）' },
  { id: 'en_US-hfc_male-medium', label: 'HFC Male', description: '英语（美国）' },
]
const currentVoiceLabel = computed(() => voices.find((v) => v.id === store.settings.ttsVoice)?.label || 'Mike')

function openNameEditor() {
  childNameDraft.value = store.child?.child_name || ''
  nameError.value = ''
  nameSheet.value = true
}

async function saveChildName() {
  const childName = childNameDraft.value.trim()
  if (!childName || [...childName].length > 20) {
    nameError.value = '宝宝姓名需为 1～20 个字符'
    return
  }
  nameSaving.value = true
  nameError.value = ''
  try {
    await api.childUpdate(store.child.child_id, { child_name: childName })
    store.child.child_name = childName
    nameSheet.value = false
  } catch (e) {
    nameError.value = e.message || '保存失败，请稍后重试'
  } finally {
    nameSaving.value = false
  }
}

function selectVoice(voice) {
  store.saveSettings({ ttsVoice: voice })
}

function previewVoice() {
  unlock()
  playUrl(api.ttsUrl('Hello, let us learn English together.', store.settings.ttsRate, store.settings.ttsVoice), { rate: store.settings.ttsRate })
}

async function logout() {
  await api.logout()
  store.resetUserData()
  localStorage.removeItem('babyeng_username')
  router.replace('/login')
}

function setBand(band) {
  store.setBand(band)
  if (store.child) {
    api.childUpdate(store.child.child_id, { age_band_override: band }).catch(() => {})
  }
}
function setLevel(lv) {
  if (store.child) {
    api.childUpdate(store.child.child_id, { level: lv }).catch(() => {})
    store.child.level = lv
  }
}
function toggleAudioOnly() {
  store.saveSettings({ audioOnly: !store.settings.audioOnly })
}
function onRate(e) {
  store.saveSettings({ ttsRate: Number(e.target.value) / 100 })
}

function askCloud() {
  if (store.settings.cloudModel) {
    store.saveSettings({ cloudModel: false })
    return
  }
  cloudSheet.value = true
}
function confirmCloud(ok) {
  if (ok) {
    store.saveSettings({ cloudModel: true, cloudConsentedAt: new Date().toISOString() })
  }
  cloudSheet.value = false
}

async function cleanup() {
  try {
    const r = await api.cleanupExpired()
    alert(`已清理 ${r.cleaned} 条过期录音，释放约 ${(r.freed_bytes / 1024 / 1024).toFixed(1)} MB`)
  } catch {
    alert('清理失败，请稍后重试')
  }
}

async function exportData() {
  try {
    const data = await api.exportData()
    const blob = new Blob([JSON.stringify(data, null, 2)], {
      type: 'application/json',
    })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = 'babyeng-backup.json'
    a.click()
    URL.revokeObjectURL(url)
  } catch {
    alert('导出失败，请稍后重试')
  }
}

async function confirmClear() {
  if (confirm('确认清空所有录音与学习记录？此操作不可撤销。')) {
    try {
      const result = await api.clearData()
      alert(`已清空 ${result.recordings_deleted} 条录音和 ${result.learning_records_deleted} 条学习记录`)
    } catch {
      alert('清空失败，数据未被删除')
    }
  }
}
</script>
