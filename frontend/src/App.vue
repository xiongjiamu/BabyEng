<template>
  <!-- 移动端优先布局：宽屏时居中套手机外壳（对应原型的 phone 外壳体验） -->
  <div class="app-frame">
    <div class="app-screen">
      <router-view v-slot="{ Component }">
        <transition name="page" mode="out-in">
          <component :is="Component" />
        </transition>
      </router-view>
    </div>
  </div>
</template>

<script setup>
// 根组件：仅负责布局容器
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
</style>
