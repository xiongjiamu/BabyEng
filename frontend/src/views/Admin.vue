<template>
  <div class="page">
    <header class="appbar">
      <router-link class="icon-btn" to="/settings" aria-label="返回">←</router-link>
      <h1>管理后台</h1><span class="icon-btn" aria-hidden="true"></span>
    </header>
    <div class="seg admin-tabs">
      <button :aria-pressed="tab === 'courses'" @click="tab = 'courses'">课程内容</button>
      <button :aria-pressed="tab === 'users'" @click="tab = 'users'">用户</button>
    </div>

    <main class="page-body pad stack-5 admin-main">
      <div v-if="message" class="banner" :class="messageType">{{ message }}</div>

      <template v-if="tab === 'users'">
        <div class="row-between"><div><h2 class="admin-heading">用户管理</h2><div class="t-mom-sm">账号密码保存在本机配置中</div></div><button class="btn btn-primary" @click="newUser">新增用户</button></div>
        <div class="list">
          <button v-for="item in users" :key="item.username" class="list-item" @click="editUser(item)">
            <span class="grow"><b>{{ item.username }}</b><span class="sub">{{ item.child_name || '尚未初始化家庭' }}</span></span>
            <span class="chip mom">{{ item.role === 'admin' ? '管理员' : '家庭用户' }}</span>
          </button>
        </div>
      </template>

      <template v-else>
        <div class="row-between"><div><h2 class="admin-heading">课程内容</h2><div class="t-mom-sm">草稿不会下发给学习端</div></div><button class="btn btn-primary" @click="newCourse">新增课程</button></div>
        <div class="seg subject-tabs">
          <button v-for="s in subjects" :key="s.id" :aria-pressed="subject === s.id" @click="selectSubject(s.id)">{{ s.label }}</button>
        </div>
        <label class="btn btn-ghost btn-block import-btn">导入 JSON<input type="file" accept="application/json,.json" @change="importJson" /></label>
        <div class="list">
          <button v-for="item in courses" :key="item.id" class="list-item" @click="editCourse(item)">
            <span class="grow"><b>{{ item.en || item.title }}</b><span class="sub">{{ item.zh || item.prompt }} · {{ item.category }}</span></span>
            <span class="chip" :class="item.review_status === 'published' ? 'kid' : 'mom'">{{ item.review_status === 'published' ? '已发布' : '草稿' }}</span>
          </button>
        </div>
      </template>
    </main>

    <div v-if="userSheet" class="overlay">
      <form class="sheet stack-4" @submit.prevent="saveUser">
        <h2 class="t-zh-lg">{{ editingUser ? '更新用户' : '新增用户' }}</h2>
        <label class="field">账号<input v-model.trim="userForm.username" :disabled="!!editingUser" required minlength="3" /></label>
        <label class="field">{{ editingUser ? '新密码' : '密码' }}<input v-model="userForm.password" type="password" required minlength="8" autocomplete="new-password" /></label>
        <label class="field">权限<select v-model="userForm.role"><option value="user">家庭用户</option><option value="admin">管理员</option></select></label>
        <button class="btn btn-primary btn-block" :disabled="saving">保存</button><button type="button" class="btn btn-ghost btn-block" @click="userSheet=false">取消</button>
      </form>
    </div>

    <div v-if="courseSheet" class="overlay">
      <form class="sheet stack-3 admin-sheet" @submit.prevent="saveCourse">
        <h2 class="t-zh-lg">{{ editingCourseId ? '编辑课程' : '新增课程' }}</h2>
        <label class="field">课程 ID<input v-model.trim="courseForm.id" :disabled="!!editingCourseId" required /></label>
        <label class="field">分类<input v-model.trim="courseForm.category" required /></label>
        <template v-if="subject === 'english'">
          <label class="field">类型<select v-model="courseForm.kind"><option value="word">单词</option><option value="sentence">情景短句</option></select></label>
          <label class="field">英文<input v-model.trim="courseForm.en" required /></label>
          <label class="field">中文<input v-model.trim="courseForm.zh" required /></label>
          <label class="field">音标<input v-model.trim="courseForm.phonetic" /></label>
          <label class="field">别名（逗号分隔）<input v-model="aliasesText" /></label>
          <label class="field">英文例句<input v-model.trim="courseForm.example_en" /></label>
          <label class="field">中文例句<input v-model.trim="courseForm.example_zh" /></label>
        </template>
        <template v-else>
          <label class="field">标题<input v-model.trim="courseForm.title" required /></label>
          <label class="field">引导语<textarea v-model.trim="courseForm.prompt" required></textarea></label>
          <label class="field">答案<input v-model.trim="courseForm.answer" required /></label>
        </template>
        <div class="row"><label class="field grow">图标<input v-model.trim="courseForm.image_emoji" /></label><label class="field grow">难度<select v-model.number="courseForm.level"><option :value="1">L1</option><option :value="2">L2</option><option :value="3">L3</option></select></label></div>
        <label class="field">状态<select v-model="courseForm.review_status"><option value="draft">草稿</option><option value="published">发布</option></select></label>
        <button class="btn btn-primary btn-block" :disabled="saving">保存课程</button><button type="button" class="btn btn-ghost btn-block" @click="courseSheet=false">取消</button>
      </form>
    </div>
  </div>
</template>

<script setup>
import { onMounted, reactive, ref } from 'vue'
import { api } from '../api'

const tab = ref('courses'), subject = ref('english'), users = ref([]), courses = ref([])
const userSheet = ref(false), courseSheet = ref(false), editingUser = ref(''), editingCourseId = ref(''), aliasesText = ref(''), saving = ref(false)
const message = ref(''), messageType = ref('ok')
const subjects = [{ id:'english',label:'英语' },{ id:'chinese',label:'语文' },{ id:'math',label:'数学' }]
const userForm = reactive({ username:'', password:'', role:'user' })
const courseForm = reactive(emptyCourse())

onMounted(async () => { await Promise.all([loadUsers(), loadCourses()]) })
async function loadUsers(){ try { users.value=(await api.adminUsers()).users||[] } catch(e){ show(e.message,'danger') } }
async function loadCourses(){ try { courses.value=(await api.adminCourses(subject.value)).items||[] } catch(e){ show(e.message,'danger') } }
async function selectSubject(value){ subject.value=value; await loadCourses() }
function newUser(){ editingUser.value=''; Object.assign(userForm,{username:'',password:'',role:'user'}); userSheet.value=true }
function editUser(item){ editingUser.value=item.username; Object.assign(userForm,{username:item.username,password:'',role:item.role}); userSheet.value=true }
async function saveUser(){ saving.value=true; try { if(editingUser.value) await api.adminUpdateUser(editingUser.value,userForm); else await api.adminCreateUser(userForm); userSheet.value=false; await loadUsers(); show('用户已保存') } catch(e){ show(e.message,'danger') } finally{ saving.value=false } }
function emptyCourse(){ return {id:'',subject:'english',kind:'word',category:'',title:'',prompt:'',answer:'',zh:'',en:'',aliases:[],phonetic:'',image_emoji:'',level:1,example_en:'',example_zh:'',mother_tip:'',review_status:'draft'} }
function newCourse(){ editingCourseId.value=''; Object.assign(courseForm,emptyCourse(),{subject:subject.value}); aliasesText.value=''; courseSheet.value=true }
function editCourse(item){ editingCourseId.value=item.id; Object.assign(courseForm,emptyCourse(),item); aliasesText.value=(item.aliases||[]).join('，'); courseSheet.value=true }
async function saveCourse(){ saving.value=true; courseForm.subject=subject.value; courseForm.aliases=aliasesText.value.split(/[，,]/).map(x=>x.trim()).filter(Boolean); try { if(editingCourseId.value) await api.adminUpdateCourse(editingCourseId.value,courseForm); else await api.adminCreateCourse(courseForm); courseSheet.value=false; await loadCourses(); show('课程已保存') } catch(e){ show(e.message,'danger') } finally{ saving.value=false } }
async function importJson(event){ const file=event.target.files?.[0]; if(!file)return; try { const parsed=JSON.parse(await file.text()); const items=Array.isArray(parsed)?parsed:parsed.items; if(!Array.isArray(items))throw new Error('JSON 应为数组或包含 items 数组'); await api.adminImportCourses(items); await loadCourses(); show(`已导入 ${items.length} 条课程`) } catch(e){ show(e.message||'导入失败','danger') } finally{ event.target.value='' } }
function show(text,type='ok'){ message.value=text; messageType.value=type; setTimeout(()=>{ if(message.value===text)message.value='' },4000) }
</script>

<style scoped>
.admin-tabs { margin: var(--sp-3) var(--sp-5) 0; }
.admin-main { width:100%;max-width:1180px;margin:0 auto; }
.admin-heading { margin:0;font-size:22px; }
.subject-tabs { width:100%; }
.import-btn { position:relative;overflow:hidden; }
.import-btn input { position:absolute;inset:0;opacity:0;cursor:pointer; }
.field { display:flex;flex-direction:column;gap:6px;font-size:14px;font-weight:700;color:var(--c-ink-2); }
.field input,.field select,.field textarea { min-height:48px;border:2px solid var(--c-line);border-radius:var(--r-md);padding:10px 12px;font:inherit;background:var(--c-surface);color:var(--c-ink); }
.field textarea { min-height:80px;resize:vertical; }
.admin-sheet { max-height:88dvh;overflow-y:auto; }
@media (min-width: 760px) {
  .admin-tabs { width:360px;margin:var(--sp-4) auto 0; }
  .admin-main > .list { display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:var(--sp-3);background:transparent; }
  .admin-main > .list .list-item { border:0;border-radius:var(--r-md);background:var(--c-surface);box-shadow:var(--shadow-1); }
  .overlay .sheet { max-width:620px;border-radius:var(--r-xl);margin:auto; }
}
</style>
