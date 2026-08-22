<template>
  <div class="page auth-page">
    <main class="auth-card stack-5">
      <div class="auth-brand">🐙</div>
      <div><h1 class="t-zh-lg">登录 BabyEng</h1><p class="t-mom">使用服务器 auth.json 中配置的家庭账号</p></div>
      <form class="stack-4" @submit.prevent="submit">
        <label class="stack-2"><span class="t-label">账号</span><input v-model.trim="username" class="auth-input" autocomplete="username" required autofocus /></label>
        <label class="stack-2"><span class="t-label">密码</span><input v-model="password" class="auth-input" type="password" autocomplete="current-password" required /></label>
        <div v-if="error" class="banner danger">{{ error }}</div>
        <button class="btn btn-primary btn-block btn-lg" :disabled="loading">{{ loading ? '正在登录…' : '登录' }}</button>
      </form>
    </main>
  </div>
</template>

<script setup>
import { ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { api } from '../api'
import { useAppStore } from '../stores/app'

const router = useRouter()
const route = useRoute()
const store = useAppStore()
const username = ref('')
const password = ref('')
const error = ref('')
const loading = ref(false)

async function submit() {
  loading.value = true
  error.value = ''
  try {
    const result = await api.login(username.value, password.value)
    localStorage.setItem('babyeng_username', result.username)
    localStorage.setItem('babyeng_role', result.role || 'user')
    store.resetUserData()
    await store.bootstrap()
    const redirect = typeof route.query.redirect === 'string' && route.query.redirect.startsWith('/') ? route.query.redirect : ''
    router.replace(redirect || (store.initialized ? '/home' : '/onboarding'))
  } catch (e) { error.value = e.message || '登录失败' } finally { loading.value = false }
}
</script>

<style scoped>
.auth-page { justify-content: center; padding: var(--sp-5); }
.auth-card { width: 100%; padding: var(--sp-7) var(--sp-5); background: var(--c-surface); border-radius: var(--r-xl); box-shadow: var(--shadow-2); }
.auth-brand { font-size: 72px; text-align: center; }
.auth-card h1, .auth-card p { margin: 0; text-align: center; }
.auth-card p { margin-top: var(--sp-2); }
.auth-input { width: 100%; min-height: 52px; padding: 0 var(--sp-4); border: 2px solid var(--c-line); border-radius: var(--r-md); background: var(--c-surface-2); font: inherit; color: var(--c-ink); }
.auth-input:focus { outline: 3px solid var(--c-primary-soft); border-color: var(--c-primary); }
</style>
