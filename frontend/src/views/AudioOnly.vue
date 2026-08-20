<template>
  <div class="page audio-screen">
    <header class="appbar">
      <router-link class="icon-btn" to="/home" aria-label="返回首页">←</router-link>
      <h1 style="font-size:18px">纯音频</h1>
      <button class="icon-btn" @click="toggleLock" aria-label="锁屏播放演示">🌙</button>
    </header>

    <!-- 普通视图（6.6：极简单页，无动画、无贴纸、无图片） -->
    <section v-if="!lockView" class="page-body pad stack-6">
      <div class="banner ok">
        <span class="ico">🎧</span>
        <span>宝宝不看屏幕，只听声音。<br><b>这段时间不计入宝宝屏幕时间。</b></span>
      </div>

      <div class="audio-word">
        <div class="en">{{ current.en }}</div>
        <div class="ipa">{{ current.phonetic }}</div>
        <div class="zh">{{ current.zh }}</div>
      </div>

      <div class="big-keys">
        <button class="bigkey play" @click="playCurrent"><span class="ico">▶</span><span>放给他听</span></button>
        <button class="bigkey rec" @click="goRecord"><span class="ico">🎤</span><span>录他跟读</span></button>
      </div>

      <div class="stepper">
        <button @click="prev">‹ 上一个</button>
        <span class="t-mom-sm">{{ idx + 1 }} / {{ list.length }} · {{ sceneName }}</span>
        <button @click="next">下一个 ›</button>
      </div>

      <div class="divider"></div>

      <div class="stack-3">
        <div class="t-label">怎么用</div>
        <p class="t-mom" style="margin:0">
          手机可以扣在桌上或攥在手里。你先听一遍、跟读一遍，再教给宝宝，宝宝跟读时按橙色键录音。
          整个过程宝宝一次屏幕都不用看。
        </p>
      </div>

      <button class="btn btn-ghost btn-block" @click="toggleAudioOnly">切回看图模式</button>

      <p class="note">
        PRD 6.6：A 段（12~24 月）默认开启此模式，把幼儿屏幕时间压到零，同时不损失任何教学闭环。
        副作用是没有画面可以掩饰，逼着发音质量必须做扎实。
      </p>
    </section>

    <!-- 锁屏播放（Media Session 演示，9.2 需真机实测） -->
    <section v-else class="page-body pad stack-5">
      <p class="t-mom center-text">锁屏后的样子</p>
      <div class="lockscreen">
        <div class="lk-sub">BabyEng · 纯音频</div>
        <div class="lk-title">{{ current.en }} {{ current.phonetic }}</div>
        <div class="lk-row">
          <span class="lk-btn">‹‹</span>
          <span class="lk-btn" style="width:56px;height:56px;background:rgba(255,255,255,.22);font-size:22px">▶</span>
          <span class="lk-btn">››</span>
          <span class="grow"></span>
          <span class="lk-sub">{{ idx + 1 }} / {{ list.length }}</span>
        </div>
      </div>
      <div class="banner warn">
        <span class="ico">⚠️</span>
        <span><b>此项需真机实测。</b>iOS 上添加到主屏幕的 PWA，后台音频行为不稳定（PRD 9.2）。若不可用，降级为黑屏播放页，而不是推翻 PWA 方案。</span>
      </div>
      <button class="btn btn-ghost btn-block" @click="toggleLock">返回</button>
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
const { playUrl, unlock } = useAudio()

const lockView = ref(false)
const idx = ref(0)
const list = ref([
  { en: 'cup', phonetic: '/kʌp/', zh: '杯子', scene: 'item_tableware' },
  { en: 'bowl', phonetic: '/boʊl/', zh: '碗', scene: 'item_tableware' },
  { en: 'spoon', phonetic: '/spuːn/', zh: '勺子', scene: 'item_tableware' },
  { en: 'plate', phonetic: '/pleɪt/', zh: '盘子', scene: 'item_tableware' },
  { en: 'apple', phonetic: '/ˈæpəl/', zh: '苹果', scene: 'item_food' },
  { en: 'banana', phonetic: '/bəˈnænə/', zh: '香蕉', scene: 'item_food' },
  { en: 'milk', phonetic: '/mɪlk/', zh: '牛奶', scene: 'item_food' },
  { en: 'egg', phonetic: '/ɛɡ/', zh: '鸡蛋', scene: 'item_food' },
])

const current = computed(() => list.value[idx.value] || list.value[0])
const sceneName = computed(() => {
  const map = { item_tableware: '餐具', item_food: '食物' }
  return map[current.value.scene] || ''
})

onMounted(async () => {
  await store.bootstrap()
  unlock()
})

function playCurrent() {
  playUrl(api.ttsUrl(current.value.en, store.settings.ttsRate, store.settings.ttsVoice), { rate: store.settings.ttsRate })
}
function next() {
  idx.value = (idx.value + 1) % list.value.length
  playCurrent()
}
function prev() {
  idx.value = (idx.value - 1 + list.value.length) % list.value.length
  playCurrent()
}
function toggleLock() {
  lockView.value = !lockView.value
}
function goRecord() {
  router.push({
    path: '/compare',
    query: {
      target_type: 'word',
      target_id: `word_${current.value.en}`,
      en: current.value.en,
      zh: current.value.zh,
      phonetic: current.value.phonetic,
      emoji: '🎧',
      from: 'audio',
    },
  })
}
function toggleAudioOnly() {
  store.saveSettings({ audioOnly: false })
  router.push('/home')
}
</script>
