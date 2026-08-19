<template>
  <nav class="tabbar">
    <router-link to="/home" :aria-current="current === 'home' ? 'page' : undefined">
      <span class="ico">🏠</span><span>首页</span>
    </router-link>
    <router-link to="/profile" :aria-current="current === 'profile' ? 'page' : undefined">
      <span class="ico">📔</span><span>我的</span>
    </router-link>
    <router-link to="/settings" :aria-current="current === 'settings' ? 'page' : undefined">
      <span class="ico">⚙</span><span>设置</span>
    </router-link>
  </nav>
</template>

<script setup>
defineProps({ current: { type: String, default: 'home' } })
</script>
