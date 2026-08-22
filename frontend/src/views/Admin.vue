<template>
  <div class="page">
    <header class="appbar">
      <router-link class="icon-btn" to="/settings" aria-label="返回">←</router-link>
      <h1>管理后台</h1><span class="icon-btn" aria-hidden="true"></span>
    </header>
    <div class="seg admin-tabs">
      <button :aria-pressed="tab === 'courses'" @click="tab = 'courses'">课程内容</button>
      <button :aria-pressed="tab === 'users'" @click="tab = 'users'">用户</button>
      <button :aria-pressed="tab === 'metrics'" @click="selectTab('metrics')">使用证据</button>
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

      <template v-else-if="tab === 'metrics'">
        <div class="row-between"><div><h2 class="admin-heading">家庭使用证据</h2><div class="t-mom-sm">仅统计本机事件，不保存提问原文</div></div><select v-model.number="metricsDays" class="period-select" @change="loadMetrics"><option :value="7">近 7 天</option><option :value="28">近 4 周</option><option :value="90">近 90 天</option></select></div>
        <div v-if="metricsLoading" class="card-flat t-mom">正在读取本地统计…</div>
        <div v-else-if="!metrics.length" class="card-flat t-mom">还没有已初始化的家庭或新口径事件。</div>
        <section v-for="family in metrics" :key="family.family_id" class="metrics-family stack-4">
          <div><h3 class="t-zh-lg">{{ family.child_name || family.mother_name || '家庭' }}</h3><div class="t-mom-sm">{{ family.tracking_started_at ? `本口径首条事件：${formatDateTime(family.tracking_started_at)}` : '本口径尚无问答事件' }}</div></div>
          <div class="metric-grid">
            <article class="metric-card"><b>{{ family.teaching_days }}</b><span>周期内闭环教学日</span><small>逐周目标见下表</small></article>
            <article class="metric-card"><b>{{ formatRate(family.completion_rate) }}</b><span>问答闭环率</span><small>{{ family.completed }}/{{ family.asks }} 次</small></article>
            <article class="metric-card"><b>{{ formatRate(family.unmatched_rate) }}</b><span>未命中率</span><small>{{ family.misses }}/{{ family.matchable_asks }} 次已识别提问</small></article>
            <article class="metric-card"><b>{{ formatRate(family.asr_success_rate) }}</b><span>语音识别成功率</span><small>{{ family.voice_asks - family.asr_failures }}/{{ family.voice_asks }} 次</small></article>
            <article class="metric-card"><b>{{ family.backend_p95_ms == null ? '—' : `${family.backend_p95_ms} ms` }}</b><span>后端处理 P95</span><small>不替代真机 A1</small></article>
            <article class="metric-card"><b>{{ formatRate(family.short_recording_rate) }}</b><span>短录音废片率</span><small>{{ family.short_recordings }}/{{ family.recording_attempts }} 次</small></article>
          </div>
          <div v-if="family.retention.evaluation_ready" class="banner" :class="family.retention.stop_loss_triggered ? 'danger' : 'ok'">
            <span>{{ family.retention.stop_loss_triggered ? '前四周周均低于 2 天，应暂停扩功能并先排查原因。' : '已具备四周证据，可继续按实际问题决定下一步。' }}</span>
            <small>前四周周均 {{ formatNumber(family.retention.first_four_weekly_average) }} 天；第 4 周 {{ family.retention.week_four_teaching_days }} 天（目标 ≥ 3 天）</small>
          </div>
          <div v-else class="banner warn"><span>四周留存尚不能判定</span><small>{{ family.retention.evaluation_date ? `最早于 ${family.retention.evaluation_date} 形成完整四周证据` : '首次问答后开始计算' }}</small></div>
          <div class="week-table" v-if="family.weeks.length">
            <div class="week-row week-head"><span>跟踪周</span><span>教学日</span><span>问答</span><span>闭环</span><span>未命中</span></div>
            <div v-for="week in family.weeks" :key="week.week_number" class="week-row"><span>第 {{ week.week_number }} 周<br><small>{{ week.week_start }}{{ week.is_complete ? '' : ' · 进行中' }}</small></span><span>{{ week.teaching_days }}</span><span>{{ week.asks }}</span><span>{{ week.completed }}</span><span>{{ week.misses }}</span></div>
          </div>
        </section>
        <section class="metrics-family stack-4">
          <div><h3 class="t-zh-lg">待补词清单</h3><div class="t-mom-sm">近 {{ metricsDays }} 天按归一化提问聚合，不显示家庭原始录音或原始文本</div></div>
          <div v-if="unmatchedLoading" class="card-flat t-mom">正在读取待补词…</div>
          <div v-else-if="!unmatched.length" class="card-flat t-mom">当前窗口没有待处理的未命中提问。</div>
          <div v-else class="unmatched-table">
            <div class="unmatched-row unmatched-head"><span>归一化提问</span><span>出现家庭</span><span>次数</span><span>最近出现</span></div>
            <div v-for="item in unmatched" :key="item.normalized_text" class="unmatched-row"><span>{{ item.normalized_text }}</span><span>{{ item.family_count }}</span><span>{{ item.hit_count }}</span><span>{{ formatDateTime(item.last_seen_at) }}</span></div>
          </div>
        </section>
        <p class="note">统计从本次事件口径上线后开始，历史学习记录不会被推测补齐。闭环教学日要求同一次问答事件关联到已保存的跟读录音；后端 P95 只用于服务端趋势定位。</p>
      </template>

      <template v-else>
        <div class="row-between"><div><h2 class="admin-heading">课程内容</h2><div class="t-mom-sm">草稿不会下发给学习端<span v-if="coursePhotoCoverage"> · 可配照片 {{ coursePhotoCoverage.with_image }}/{{ coursePhotoCoverage.supported }}（{{ formatRate(coursePhotoCoverage.rate) }}）</span></div></div><button class="btn btn-primary" @click="newCourse">新增课程</button></div>
        <div class="seg subject-tabs">
          <button v-for="s in subjects" :key="s.id" :aria-pressed="subject === s.id" @click="selectSubject(s.id)">{{ s.label }}</button>
        </div>
        <label class="btn btn-ghost btn-block import-btn">导入 JSON<input type="file" accept="application/json,.json" @change="importJson" /></label>
        <div class="list">
          <button v-for="item in courses" :key="item.id" class="list-item" @click="editCourse(item)">
            <span class="grow"><b>{{ item.en || item.title }}</b><span class="sub">{{ item.zh || item.prompt }} · {{ item.category }}</span></span>
            <span class="course-meta"><span class="chip" :class="item.review_status === 'published' ? 'kid' : 'mom'">{{ item.review_status === 'published' ? '已发布' : '草稿' }}</span><span v-if="item.kind !== 'sentence'" class="chip" :class="item.image_exists ? 'photo-ready' : 'photo-missing'">{{ item.image_exists ? '有照片' : '待照片' }}</span><span v-else class="chip photo-na">无照片</span></span>
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
          <label class="field">生活场景<select v-model="courseForm.scene"><option value="morning">起床</option><option value="meal">吃饭</option><option value="play">玩耍</option><option value="dressing">穿衣</option><option value="outing">出门</option><option value="bedtime">睡前</option></select></label>
          <label class="field">准备材料<textarea v-model.trim="courseForm.materials"></textarea></label>
          <label class="field">妈妈照着说<textarea v-model.trim="courseForm.parent_script"></textarea></label>
          <label class="field">A 段动作（12～24 月）<textarea v-model.trim="courseForm.child_action_a"></textarea></label>
          <label class="field">B 段动作（24～36 月）<textarea v-model.trim="courseForm.child_action_b"></textarea></label>
          <label class="field">观察点<textarea v-model.trim="courseForm.observe_for"></textarea></label>
          <label class="field">安全提醒<textarea v-model.trim="courseForm.safety_note"></textarea></label>
          <div class="field">材料标签<div class="tag-grid"><button v-for="item in materialOptions" :key="item.id" type="button" class="tag-choice" :aria-pressed="courseForm.material_tags.includes(item.id)" @click="toggleCourseTag('material_tags', item.id)">{{ item.label }}</button></div></div>
          <div class="field">兴趣标签<div class="tag-grid"><button v-for="item in interestOptions" :key="item.id" type="button" class="tag-choice" :aria-pressed="courseForm.interest_tags.includes(item.id)" @click="toggleCourseTag('interest_tags', item.id)">{{ item.label }}</button></div></div>
        </template>
        <div class="row"><label class="field grow">图标<input v-model.trim="courseForm.image_emoji" /></label><label class="field grow">难度<select v-model.number="courseForm.level"><option :value="1">L1</option><option :value="2">L2</option><option :value="3">L3</option></select></label></div>
        <div v-if="editingCourseId && canHaveImage" class="field">
          <span>自家实物照片</span>
          <ContentImage class="course-image-preview" :kind="courseImageKind" :target-id="courseForm.id" :emoji="courseForm.image_emoji" :alt="courseForm.zh || courseForm.title" :version="imageVersion" @loaded="courseImageExists=true" @fallback="courseImageExists=false" />
          <label class="btn btn-ghost btn-block import-btn">{{ courseImageExists ? '替换照片' : '上传照片' }}<input type="file" accept="image/jpeg,image/png,image/webp" @change="uploadCourseImage" /></label>
          <button v-if="courseImageExists" type="button" class="btn btn-ghost btn-block" :disabled="savingImage" @click="deleteCourseImage">删除照片并恢复图标</button>
          <span class="t-mom-sm">仅支持 JPEG、PNG、WebP，最大 5MB。照片保存在自己的服务器上。</span>
        </div>
        <label class="field">状态<select v-model="courseForm.review_status"><option value="draft">草稿</option><option value="published">发布</option></select></label>
        <button class="btn btn-primary btn-block" :disabled="saving">保存课程</button><button type="button" class="btn btn-ghost btn-block" @click="courseSheet=false">取消</button>
      </form>
    </div>
  </div>
</template>

<script setup>
import { computed, onMounted, reactive, ref } from 'vue'
import { api } from '../api'
import ContentImage from '../components/ContentImage.vue'

const tab = ref('courses'), subject = ref('english'), users = ref([]), courses = ref([])
const coursePhotoCoverage = ref(null)
const userSheet = ref(false), courseSheet = ref(false), editingUser = ref(''), editingCourseId = ref(''), aliasesText = ref(''), saving = ref(false)
const message = ref(''), messageType = ref('ok')
const metrics = ref([]), metricsDays = ref(28), metricsLoading = ref(false), unmatched = ref([]), unmatchedLoading = ref(false)
const savingImage = ref(false), courseImageExists = ref(false), imageVersion = ref(Date.now())
const subjects = [{ id:'english',label:'英语' },{ id:'chinese',label:'语文' },{ id:'math',label:'数学' }]
const materialOptions = [{id:'household_objects',label:'日常物品'},{id:'toys_blocks',label:'玩具积木'},{id:'food_tableware',label:'食物餐具'},{id:'clothing',label:'衣物'},{id:'movement_space',label:'活动空间'}]
const interestOptions = [{id:'animals',label:'动物'},{id:'music',label:'音乐儿歌'},{id:'vehicles',label:'车辆'},{id:'building',label:'搭建'},{id:'food',label:'食物'},{id:'outdoors',label:'户外'},{id:'movement',label:'动作模仿'}]
const userForm = reactive({ username:'', password:'', role:'user' })
const courseForm = reactive(emptyCourse())

onMounted(async () => { await Promise.all([loadUsers(), loadCourses()]) })
async function loadUsers(){ try { users.value=(await api.adminUsers()).users||[] } catch(e){ show(e.message,'danger') } }
async function loadCourses(){ try { const result=await api.adminCourses(subject.value); courses.value=result.items||[]; coursePhotoCoverage.value=result.photo_coverage||null } catch(e){ show(e.message,'danger') } }
async function selectSubject(value){ subject.value=value; await loadCourses() }
async function selectTab(value){ tab.value=value; if(value==='metrics')await loadMetrics() }
async function loadMetrics(){ metricsLoading.value=true; unmatchedLoading.value=true; try { const [metricResult, unmatchedResult] = await Promise.all([api.adminUsageMetrics(metricsDays.value), api.adminUnmatched(metricsDays.value)]); metrics.value=metricResult.families||[]; unmatched.value=unmatchedResult.items||[] } catch(e){ show(e.message,'danger') } finally { metricsLoading.value=false; unmatchedLoading.value=false } }
function formatRate(value){ return value == null ? '—' : `${Math.round(value*100)}%` }
function formatNumber(value){ return value == null ? '—' : Number(value).toFixed(1) }
function formatDateTime(value){ const date=new Date(value); return Number.isNaN(date.getTime())?value:date.toLocaleString() }
function newUser(){ editingUser.value=''; Object.assign(userForm,{username:'',password:'',role:'user'}); userSheet.value=true }
function editUser(item){ editingUser.value=item.username; Object.assign(userForm,{username:item.username,password:'',role:item.role}); userSheet.value=true }
async function saveUser(){ saving.value=true; try { if(editingUser.value) await api.adminUpdateUser(editingUser.value,userForm); else await api.adminCreateUser(userForm); userSheet.value=false; await loadUsers(); show('用户已保存') } catch(e){ show(e.message,'danger') } finally{ saving.value=false } }
function emptyCourse(){ return {id:'',subject:'english',kind:'word',category:'',title:'',prompt:'',answer:'',zh:'',en:'',aliases:[],phonetic:'',image_emoji:'',level:1,example_en:'',example_zh:'',mother_tip:'',scene:'play',materials:'',parent_script:'',child_action_a:'',child_action_b:'',observe_for:'',safety_note:'',material_tags:[],interest_tags:[],review_status:'draft'} }
const canHaveImage = computed(() => subject.value !== 'english' || courseForm.kind === 'word')
const courseImageKind = computed(() => subject.value === 'english' ? 'word' : 'activity')
function newCourse(){ editingCourseId.value=''; courseImageExists.value=false; Object.assign(courseForm,emptyCourse(),{subject:subject.value}); aliasesText.value=''; courseSheet.value=true }
function editCourse(item){ editingCourseId.value=item.id; courseImageExists.value=false; imageVersion.value=Date.now(); Object.assign(courseForm,emptyCourse(),item); aliasesText.value=(item.aliases||[]).join('，'); courseSheet.value=true }
async function saveCourse(){ saving.value=true; courseForm.subject=subject.value; courseForm.aliases=aliasesText.value.split(/[，,]/).map(x=>x.trim()).filter(Boolean); try { if(editingCourseId.value) await api.adminUpdateCourse(editingCourseId.value,courseForm); else await api.adminCreateCourse(courseForm); courseSheet.value=false; await loadCourses(); show('课程已保存') } catch(e){ show(e.message,'danger') } finally{ saving.value=false } }
function toggleCourseTag(key,value){ const list=courseForm[key]; courseForm[key]=list.includes(value)?list.filter(item=>item!==value):[...list,value] }
async function uploadCourseImage(event){ const file=event.target.files?.[0]; event.target.value=''; if(!file)return; if(file.size>5*1024*1024){show('图片不能超过 5MB','danger');return} const replace=courseImageExists.value; if(replace&&!confirm('确定用新照片替换当前照片吗？'))return; savingImage.value=true; try{const result=await api.adminUploadContentImage(courseImageKind.value,courseForm.id,file,replace);courseImageExists.value=true;imageVersion.value=result.version||Date.now();show('照片已保存')}catch(e){show(e.message,'danger')}finally{savingImage.value=false} }
async function deleteCourseImage(){ if(!confirm('确定删除这张照片并恢复使用图标吗？'))return; savingImage.value=true; try{await api.adminDeleteContentImage(courseImageKind.value,courseForm.id);courseImageExists.value=false;imageVersion.value=Date.now();show('照片已删除')}catch(e){show(e.message,'danger')}finally{savingImage.value=false} }
async function importJson(event){ const file=event.target.files?.[0]; if(!file)return; try { const parsed=JSON.parse(await file.text()); const items=Array.isArray(parsed)?parsed:parsed.items; if(!Array.isArray(items))throw new Error('JSON 应为数组或包含 items 数组'); await api.adminImportCourses(items); await loadCourses(); show(`已导入 ${items.length} 条课程`) } catch(e){ show(e.message||'导入失败','danger') } finally{ event.target.value='' } }
function show(text,type='ok'){ message.value=text; messageType.value=type; setTimeout(()=>{ if(message.value===text)message.value='' },4000) }
</script>

<style scoped>
.admin-tabs { margin: var(--sp-3) var(--sp-5) 0; }
.admin-main { width:100%;max-width:1180px;margin:0 auto; }
.admin-heading { margin:0;font-size:22px; }
.course-meta { display:flex;flex-direction:column;align-items:flex-end;gap:4px; }
.photo-ready,.photo-missing,.photo-na { font-size:11px; }
.photo-ready { background:var(--c-kid-soft);color:var(--c-kid); }
.photo-missing { background:var(--c-mom-soft);color:var(--c-mom); }
.photo-na { background:var(--c-surface-2);color:var(--c-ink-3); }
.subject-tabs { width:100%; }
.import-btn { position:relative;overflow:hidden; }
.import-btn input { position:absolute;inset:0;opacity:0;cursor:pointer; }
.field { display:flex;flex-direction:column;gap:6px;font-size:14px;font-weight:700;color:var(--c-ink-2); }
.field input,.field select,.field textarea { min-height:48px;border:2px solid var(--c-line);border-radius:var(--r-md);padding:10px 12px;font:inherit;background:var(--c-surface);color:var(--c-ink); }
.field textarea { min-height:80px;resize:vertical; }
.admin-sheet { max-height:88dvh;overflow-y:auto; }
.course-image-preview { width:180px;height:180px;border-radius:var(--r-lg);background:var(--c-surface-2);align-self:center;font-size:72px; }
.tag-grid { display:flex;flex-wrap:wrap;gap:8px; }
.tag-choice { border:1px solid var(--c-line);border-radius:999px;background:var(--c-surface-2);padding:8px 12px;font:inherit;color:var(--c-ink-2); }
.tag-choice[aria-pressed="true"] { border-color:var(--c-mom);background:var(--c-mom-soft);color:var(--c-mom);font-weight:800; }
.period-select { min-height:42px;border:2px solid var(--c-line);border-radius:var(--r-md);padding:0 10px;background:var(--c-surface);color:var(--c-ink); }
.metrics-family { padding:var(--sp-4);border-radius:var(--r-lg);background:var(--c-surface);box-shadow:var(--shadow-1); }
.metrics-family h3 { margin:0; }
.metrics-family .banner { display:flex;flex-direction:column;gap:4px; }
.metric-grid { display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:var(--sp-3); }
.metric-card { display:flex;flex-direction:column;gap:4px;padding:var(--sp-4);border-radius:var(--r-md);background:var(--c-surface-2); }
.metric-card b { font-size:24px;color:var(--c-mom); }
.metric-card span { font-weight:800; }
.metric-card small { color:var(--c-ink-3); }
.week-table { overflow-x:auto;border:1px solid var(--c-line);border-radius:var(--r-md); }
.week-row { display:grid;grid-template-columns:minmax(100px,1.4fr) repeat(4,minmax(58px,1fr));min-width:420px;padding:10px 12px;border-top:1px solid var(--c-line);font-size:13px;text-align:center; }
.week-row:first-child { border-top:0; }
.week-head { font-weight:800;background:var(--c-surface-2); }
.unmatched-table { overflow-x:auto;border:1px solid var(--c-line);border-radius:var(--r-md); }
.unmatched-row { display:grid;grid-template-columns:minmax(130px,1.6fr) repeat(3,minmax(80px,1fr));min-width:500px;padding:10px 12px;border-top:1px solid var(--c-line);font-size:13px;gap:8px;align-items:center; }
.unmatched-row:first-child { border-top:0; }
.unmatched-head { font-weight:800;background:var(--c-surface-2); }
@media (min-width: 760px) {
  .admin-tabs { width:540px;margin:var(--sp-4) auto 0; }
  .admin-main > .list { display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:var(--sp-3);background:transparent; }
  .admin-main > .list .list-item { border:0;border-radius:var(--r-md);background:var(--c-surface);box-shadow:var(--shadow-1); }
  .overlay .sheet { max-width:620px;border-radius:var(--r-xl);margin:auto; }
  .metric-grid { grid-template-columns:repeat(3,minmax(0,1fr)); }
}
</style>
