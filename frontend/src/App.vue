<template>
  <!-- 移动端优先布局：宽屏时居中套手机外壳（对应原型的 phone 外壳体验） -->
  <div class="app-frame" :class="{ 'admin-frame': isAdminPage }">
    <div class="app-screen" :class="{ 'admin-screen': isAdminPage }">
      <section v-if="showScreenWrapUp" class="screen-wrap-up page-body pad center stack-6">
        <div class="wrap-up-emoji">🌙</div>
        <h1 class="t-zh-lg">这次先学到这里吧</h1>
        <p class="t-mom center-text">
          {{ store.screenExceeded ? '今天的看图时间已经到了。' : '这一轮已经专心学了好一会儿。' }}
          放下手机，和宝宝一起找找刚才见过的东西。
        </p>
        <router-link class="btn btn-primary btn-block btn-lg" to="/profile">查看今日小结</router-link>
        <router-link class="btn-quiet btn-block" to="/home">回首页</router-link>
      </section>
      <router-view v-else v-slot="{ Component }">
        <transition name="page" mode="out-in">
          <component :is="Component" />
        </transition>
      </router-view>
    </div>
  </div>
</template>

<script setup>
import { computed, onBeforeUnmount, onMounted, watch } from 'vue'
import { useRoute } from 'vue-router'
import { api } from './api'
import { useAppStore } from './stores/app'

const route = useRoute()
const store = useAppStore()
const isAdminPage = computed(() => route.meta.admin === true)
const isChildScreen = computed(() => route.meta.childScreen === true)
const showScreenWrapUp = computed(() => isChildScreen.value && (store.screenExceeded || store.sessionExceeded))

let timer = null
let pendingSeconds = 0
let flushing = false

function shouldCount() {
  return isChildScreen.value && !showScreenWrapUp.value && document.visibilityState === 'visible' && !!store.childId
}

async function flushScreenTime() {
  if (flushing || pendingSeconds < 1 || !store.childId) return
  flushing = true
  const batch = Math.min(60, pendingSeconds)
  pendingSeconds -= batch
  try {
    const result = await api.recordScreenTime(store.childId, batch)
    // 服务端结果不含尚未提交的本地秒数，合并后保持界面连续。
    store.setScreenTimeToday(result.screen_sec_today + pendingSeconds)
  } catch {
    pendingSeconds += batch
  } finally {
    flushing = false
  }
}

onMounted(async () => {
  await store.bootstrap()
  if (store.childId) {
    try {
      const summary = await api.progressSummary(store.childId)
      store.setScreenTimeToday(summary.screen_sec_today)
    } catch { /* 离线时从本次会话开始计时 */ }
  }
  timer = window.setInterval(() => {
    if (!shouldCount()) return
    pendingSeconds += 1
    store.tickScreen(1)
    if (pendingSeconds >= 15) flushScreenTime()
  }, 1000)
})

watch(isChildScreen, (active, wasActive) => {
  if (active && !wasActive) store.resetScreenSession()
  if (!active && wasActive) flushScreenTime()
})

onBeforeUnmount(() => {
  if (timer) window.clearInterval(timer)
  flushScreenTime()
})
</script>

<style>
.app-frame {
  min-height: 100dvh;
  display: flex;
  justify-content: center;
  background: #F0E7DC;
}
.app-screen {
  width: 100%;
  max-width: 480px;
  height: 100dvh;
  min-height: 100dvh;
  background: var(--c-bg);
  position: relative;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  box-shadow: 0 0 40px rgba(38, 23, 11, 0.08);
}
@media (min-width: 520px) {
  .app-frame { padding: 24px 0; }
  .app-screen {
    border: 10px solid #2A2118;
    border-radius: 46px;
    overflow: hidden;
    min-height: min(880px, calc(100dvh - 48px));
    height: min(880px, calc(100dvh - 48px));
  }
}
.page-enter-active, .page-leave-active { transition: opacity var(--t-enter) var(--ease); }
.page-enter-from, .page-leave-to { opacity: 0; }
.app-frame.admin-frame { padding: 0; background: #F4F6F8; }
.app-screen.admin-screen { max-width: none; min-height: 100dvh; height: 100dvh; box-shadow: none; background: #F4F6F8; }
.screen-wrap-up { flex: 1; background: var(--c-bg); }
.wrap-up-emoji { font-size: 82px; line-height: 1; }
@media (min-width: 520px) {
  .app-screen.admin-screen { border: 0; border-radius: 0; min-height: 100dvh; height: 100dvh; }
}
</style>
