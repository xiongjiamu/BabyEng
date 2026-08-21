<template>
  <!-- 移动端优先布局：宽屏时居中套手机外壳（对应原型的 phone 外壳体验） -->
  <div class="app-frame" :class="{ 'admin-frame': isAdminPage }">
    <div class="app-screen" :class="{ 'admin-screen': isAdminPage }">
      <router-view v-slot="{ Component }">
        <transition name="page" mode="out-in">
          <component :is="Component" />
        </transition>
      </router-view>
    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue'
import { useRoute } from 'vue-router'

const route = useRoute()
const isAdminPage = computed(() => route.meta.admin === true)
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
@media (min-width: 520px) {
  .app-screen.admin-screen { border: 0; border-radius: 0; min-height: 100dvh; height: 100dvh; }
}
</style>
