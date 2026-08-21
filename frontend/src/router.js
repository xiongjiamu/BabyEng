import { createRouter, createWebHistory } from 'vue-router'
import { authToken } from './api'

// 12 个页面，与原型一一对应（PRD 5.2 页面清单）
const routes = [
  { path: '/login', name: 'login', component: () => import('./views/Login.vue'), meta: { title: '登录', public: true } },
  { path: '/', redirect: '/home' },
  { path: '/onboarding', name: 'onboarding', component: () => import('./views/Onboarding.vue'), meta: { title: '首次引导' } },
  { path: '/home', name: 'home', component: () => import('./views/Home.vue'), meta: { title: '首页' } },
  { path: '/ask', name: 'ask', component: () => import('./views/Ask.vue'), meta: { title: '问一问' } },
  { path: '/compare', name: 'compare', component: () => import('./views/Compare.vue'), meta: { title: '跟读' } },
  { path: '/audio', name: 'audio', component: () => import('./views/AudioOnly.vue'), meta: { title: '纯音频' } },
  { path: '/settings', name: 'settings', component: () => import('./views/Settings.vue'), meta: { title: '设置' } },
  { path: '/states', name: 'states', component: () => import('./views/States.vue'), meta: { title: '异常状态' } },
  { path: '/learn', name: 'learn', component: () => import('./views/Learn.vue'), meta: { title: '学一学' } },
  { path: '/word-learn', name: 'word-learn', component: () => import('./views/WordLearn.vue'), meta: { title: '单词学习' } },
  { path: '/subject-learn/:subject', name: 'subject-learn', component: () => import('./views/SubjectLearn.vue'), meta: { title: '启蒙学习' } },
  { path: '/review', name: 'review', component: () => import('./views/Review.vue'), meta: { title: '复习' } },
  { path: '/sentences', name: 'sentences', component: () => import('./views/Sentences.vue'), meta: { title: '情景短句' } },
  { path: '/profile', name: 'profile', component: () => import('./views/Profile.vue'), meta: { title: '我的' } },
]

const router = createRouter({
  history: createWebHistory(),
  routes,
})

router.beforeEach((to) => {
  if (!to.meta.public && !authToken()) return { name: 'login', query: { redirect: to.fullPath } }
  if (to.name === 'login' && authToken()) return { name: 'home' }
  document.title = `${to.meta.title || 'BabyEng'} · BabyEng`
})

export default router
