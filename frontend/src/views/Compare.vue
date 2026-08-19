<template>
  <div class="page">
    <header class="appbar">
      <router-link class="icon-btn" :to="backTo" aria-label="返回">←</router-link>
      <h1>跟读</h1>
      <span class="icon-btn" aria-hidden="true"></span>
    </header>

    <!-- ============ 1. 准备录音 ============ -->
    <section v-show="view === 'ready'" class="page-body pad stack-8">
      <div class="card target">
        <span class="photo sm"><span class="emoji">{{ emoji }}</span></span>
        <div class="grow">
          <div class="t-word-en-s">{{ en }}</div>
          <div class="t-phonetic">{{ phonetic || '' }}</div>
        </div>
        <button class="play-std" aria-label="播放标准音" @click="playStd(store.settings.ttsRate)">▶</button>
      </div>

      <p class="t-mom center-text">先让宝宝听一遍标准音，<br>你自己也跟着念一次，再让他跟读。</p>

      <div class="rec-zone" :class="{ 'band-a': isBandA }">
        <button class="record" :class="{ 'is-recording': rec.state === 'recording' }" @click="toggleRecord" aria-label="点一下开始录音">
          <span v-if="rec.state !== 'recording'" class="ico">🎤</span>
          <span v-else class="wave"><i></i><i></i><i></i><i></i><i></i></span>
        </button>
        <div class="rec-caption">{{ recCaption }}</div>
      </div>

      <p class="note">
        <b>{{ isBandA ? 'A 段（12~24 月）' : 'B 段（24~36 月）' }}</b>：
        {{ isBandA ? '按钮靠向母亲惯用手侧，由母亲代为点按，宝宝只管开口。' : '按钮居中，宝宝可自行点按。' }}
        PRD 6.2 用「点按 + 静音自动停」而非「按住说话」——持续按压要到 2.5 岁前后才稳定。
      </p>
    </section>

    <!-- ============ 2. 录音中 ============ -->
    <section v-show="view === 'recording'" class="page-body pad center stack-8">
      <div class="stack-3" style="text-align:center">
        <div class="t-word-en-s">{{ en }}</div>
        <div class="t-phonetic">{{ phonetic || '' }}</div>
      </div>
      <div class="rec-zone">
        <button class="record is-recording" @click="toggleRecord" aria-label="再点一次结束">
          <span class="wave"><i></i><i></i><i></i><i></i><i></i></span>
        </button>
        <div class="timer">{{ fmt(rec.durationMs) }}</div>
        <div class="rec-caption" style="color:var(--c-ink-3);font-size:var(--fs-mom)">安静 1.5 秒会自动停</div>
      </div>
    </section>

    <!-- ============ 3. 录音过短 ============ -->
    <section v-show="view === 'tooshort'" class="page-body pad center stack-6 enter">
      <div style="text-align:center">
        <div style="font-size:72px">🙊</div>
        <h2 class="t-zh-lg" style="margin:var(--sp-4) 0 var(--sp-2)">这次没录到</h2>
        <p class="t-mom">好像只碰了一下，再来一次</p>
      </div>
      <button class="btn btn-primary btn-block btn-lg" @click="view = 'ready'">再录一次</button>
      <p class="note">PRD 5.4 / 6.2：时长 &lt; 0.5s 不入库、不计入学习记录。废片率是过程指标之一，目标 &lt; 5%。</p>
    </section>

    <!-- ============ 4. 双轨回放 + 母亲标记 ============ -->
    <section v-show="view === 'playback'" class="page-body pad stack-6 enter">
      <div class="stack-2" style="text-align:center">
        <div class="t-word-en-s">{{ en }}</div>
        <div class="t-phonetic">{{ phonetic || '' }}</div>
      </div>

      <div class="track-pair">
        <div>
          <div class="track-label"><span>🔊</span><span>标准发音</span></div>
          <div class="playbar kid">
            <button class="play" aria-label="播放标准音" @click="playStd(store.settings.ttsRate)">▶</button>
            <span class="track"><i style="width:100%"></i></span>
            <span class="t-mom-sm">—</span>
          </div>
        </div>
        <div>
          <div class="track-label"><span>👶</span><span>宝宝的声音</span></div>
          <div class="playbar">
            <button class="play" aria-label="播放宝宝录音" @click="playKid">{{ kidPlaying ? '⏸' : '▶' }}</button>
            <span class="track"><i style="width:100%"></i></span>
            <span class="t-mom-sm">{{ fmt(kidDuration) }}</span>
          </div>
        </div>
      </div>

      <div class="divider"></div>

      <div class="stack-3">
        <div class="t-mom" style="font-weight:700;color:var(--c-ink)">你觉得这次怎么样？</div>
        <div class="mark-row">
          <button class="mark got" @click="mark('got_it')"><span class="ico">🌟</span><span>学会了</span></button>
          <button class="mark keep" @click="mark('keep_trying')"><span class="ico">🔁</span><span>再练练</span></button>
        </div>
        <p class="t-mom-sm">你的耳朵比任何算法都准。这个标记只用来决定明天先复习哪个词。</p>
      </div>

      <div class="row" style="justify-content:center;gap:var(--sp-5)">
        <button class="btn-quiet" @click="view = 'ready'">↺ 重录</button>
        <button class="btn-quiet" @click="backToAsk">跳过 →</button>
      </div>

      <p class="note">PRD 4.3 修订：<b>不显示任何相似度分数</b>。改由母亲一键标记，这个标记正是掌握度算法的主信号（权重 0.55）。</p>
    </section>

    <!-- ============ 5. 鼓励弹窗 ============ -->
    <div v-if="praise.show" class="overlay center">
      <div class="dialog praise">
        <div class="face">{{ praise.face }}</div>
        <div class="t-word-en-s" style="margin-top:var(--sp-3)">{{ praise.en }}</div>
        <div class="t-zh" style="color:var(--c-ink-2);margin-top:var(--sp-2)">{{ praise.zh }}</div>
        <div class="stack-3" style="margin-top:var(--sp-6)">
          <router-link class="btn btn-primary btn-block btn-lg" to="/ask">再问一个</router-link>
          <button class="btn-quiet" @click="praise.show = false">留在这一页</button>
        </div>
        <p class="note" style="margin-top:var(--sp-5);text-align:left">无论发音如何都鼓励，文案随机避免机械重复（PRD 6.3）。</p>
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed, onMounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useAppStore } from '../stores/app'
import { api } from '../api'
import { useAudio } from '../composables/useAudio'
import { useRecorder } from '../composables/useRecorder'

const route = useRoute()
const router = useRouter()
const store = useAppStore()
const { playUrl, playBlob, unlock } = useAudio()
const rec = useRecorder()

const view = ref('ready')
const kidBlob = ref(null)
const kidDuration = ref(0)
const kidPlaying = ref(false)
const praise = ref({ show: false, face: '🎉', en: 'Great job!', zh: '真棒！' })

const targetType = computed(() => route.query.target_type || 'word')
const targetId = computed(() => route.query.target_id || '')
const en = computed(() => route.query.en || 'cup')
const zh = computed(() => route.query.zh || '杯子')
const phonetic = computed(() => route.query.phonetic || '')
const emoji = computed(() => route.query.emoji || '☕')
const isBandA = computed(() => store.isBandA)
const recCaption = computed(() => (isBandA.value ? '妈妈点一下，让宝宝说' : '点一下，跟我读'))
const backTo = computed(() => (route.query.from === 'audio' ? '/audio' : '/ask'))
const backToAsk = () => router.push('/ask')

const PRAISE = [
  ['🎉', 'Great job!', '真棒！'],
  ['👏', 'Well done!', '做得好！'],
  ['⭐️', 'You did it!', '你做到啦！'],
  ['🌈', 'Good try!', '试得不错！'],
]

onMounted(async () => {
  await store.bootstrap()
  unlock() // 进入本页先解锁音频上下文（iOS 限制）
})

function fmt(ms) {
  return `0:${String(Math.floor((ms || 0) / 1000)).padStart(2, '0')}`
}

function playStd(rate) {
  playUrl(api.ttsUrl(en.value, rate), { rate })
}

async function toggleRecord() {
  if (rec.state === 'recording') {
    const r = await rec.onStop()
    if (!r) {
      // 录音过短：不入库（5.4）
      view.value = 'tooshort'
      return
    }
    kidBlob.value = r.blob
    kidDuration.value = r.durationMs
    view.value = 'playback'
    return
  }
  try {
    await rec.start()
    view.value = 'recording'
  } catch {
    store.setMicPermission('denied')
  }
}

async function playKid() {
  if (!kidBlob.value) return
  kidPlaying.value = true
  await playBlob(kidBlob.value)
  kidPlaying.value = false
}

async function mark(motherMark) {
  // 上传录音（落盘即传，PRD 9.2：不在前端长期堆积）
  try {
    if (kidBlob.value) {
      await api.uploadRecording(kidBlob.value, `rec.${rec.ext || 'webm'}`, {
        childId: store.childId,
        targetType: targetType.value,
        targetId: targetId.value,
        durationMs: kidDuration.value,
      })
    }
  } catch {
    // PRD 9.2 / 5.4：录音落盘即上传、不前端堆积；上传失败不缓存死数据，
    // 直接提示重录，避免「假重试队列」存了一堆永远恢复不了的 metadata
    alert('网络不太好，这条录音没传上去，再录一次吧')
    view.value = 'ready'
    kidBlob.value = null
    return
  }
  // 学习记录 + 母亲标记（掌握度主信号）
  try {
    await api.recordLearning({
      child_id: store.childId,
      target_type: targetType.value,
      target_id: targetId.value,
      action: 'learn',
      mother_mark: motherMark,
    })
  } catch { /* 学习记录失败不阻断鼓励 */ }

  // 鼓励弹窗（无论发音如何都鼓励，6.3）
  const p = PRAISE[Math.floor(Math.random() * PRAISE.length)]
  praise.value = { show: true, face: p[0], en: p[1], zh: p[2] }
}
</script>
