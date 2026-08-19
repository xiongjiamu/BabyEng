<template>
  <div class="page">
    <header class="appbar">
      <router-link class="icon-btn" to="/home" aria-label="返回">←</router-link>
      <h1>异常状态</h1>
      <span class="icon-btn" aria-hidden="true"></span>
    </header>

    <div class="page-body pad stack-6" style="padding-top:0">
      <p class="t-mom" style="margin:0">以下是七种异常态的界面表现（PRD 5.4）。真实使用中按触发条件自动出现。</p>

      <!-- 1 麦克风权限被拒 -->
      <div class="card stack-4">
        <div class="banner warn">
          <span class="ico">🎙</span>
          <span class="grow"><b>麦克风没打开</b>，现在只能打字提问</span>
          <button class="chip" style="background:#fff">去开启</button>
        </div>
        <p class="t-mom" style="margin:0">应用照常可用，只是不能语音提问、也不能录宝宝的声音。要恢复，按下面三步：</p>
        <ol class="guide" style="margin:0;padding-left:0;list-style:none">
          <li><span class="n">1</span><span>打开手机「设置」</span></li>
          <li><span class="n">2</span><span>找到浏览器 → 网站设置 → 麦克风</span></li>
          <li><span class="n">3</span><span>把 BabyEng 改成「允许」</span></li>
        </ol>
      </div>

      <!-- 2 非 HTTPS -->
      <div class="card stack-3">
        <div class="big-ico" style="font-size:44px">🔓</div>
        <h3 class="t-zh-lg" style="margin:0">这个地址没法录音</h3>
        <div>
          <div class="t-label" style="margin-bottom:6px">你现在访问的</div>
          <div class="addr" style="color:var(--c-danger)">{{ locationHref }}</div>
        </div>
        <div>
          <div class="t-label" style="margin-bottom:6px">换成这个</div>
          <div class="addr" style="color:var(--c-ok)">https://babyeng.home.lan</div>
        </div>
        <p class="note" style="margin:0">浏览器只在加密地址下才允许用麦克风（PRD 9.2 / 5.4）。</p>
      </div>

      <!-- 3 网络中断 -->
      <div class="card stack-4">
        <div class="banner warn"><span class="ico">📶</span><span><b>连不上服务器</b>，已缓存的词还能用</span></div>
        <p class="t-mom" style="margin:0">已缓存的词条发音离线也能听、也能录。宝宝刚才的录音已存好，网络恢复后自动补传。</p>
        <p class="note" style="margin:0">PRD 5.4 / 9.10：离线不等于不可用。高频词的发音音频在前端预缓存，录音落盘后进重试队列。</p>
      </div>

      <!-- 4 服务未就绪 -->
      <div class="card stack-4">
        <div class="banner info"><span class="ico">⏳</span><span><b>正在启动，大约还要 1 分钟</b><br>发音和识别暂时不可用</span></div>
        <div class="stack-3">
          <div class="row-between"><span class="t-mom" style="font-weight:700;color:var(--c-ink)">发音服务 TTS</span><span class="chip ok">✓ 就绪</span></div>
          <div class="row-between"><span class="t-mom" style="font-weight:700;color:var(--c-ink)">识别服务 ASR</span><span class="chip warn">启动中</span></div>
          <div class="progress"><i style="width:70%"></i></div>
        </div>
        <p class="note" style="margin:0">推理服务设 healthcheck，后端等就绪；就绪后提示条自动解除，不需要母亲手动刷新。</p>
      </div>

      <!-- 5 录音过短 -->
      <div class="card center-text stack-3">
        <div class="big-ico" style="font-size:44px">🙊</div>
        <h3 class="t-zh-lg" style="margin:0">这次没录到</h3>
        <p class="t-mom" style="margin:0">好像只碰了一下，再来一次</p>
        <p class="note" style="margin:0">PRD 6.2：&lt; 0.5s 不入库、不计学习记录。这条错误提示不带任何责备语气——按错的多半是孩子。</p>
      </div>

      <!-- 6 磁盘将满 -->
      <div class="card stack-4">
        <div class="banner warn"><span class="ico">💾</span><span><b>存储快满了</b><br>再不清理，新的录音可能存不下</span></div>
        <div class="stack-3">
          <div class="row-between"><span class="t-mom" style="font-weight:700;color:var(--c-ink)">已用 18.6 GB / 20 GB</span><span class="t-mom-sm">93%</span></div>
          <div class="meter"><i style="width:93%"></i></div>
          <div class="row-between"><span class="t-mom">跟读录音</span><span class="t-mom" style="font-weight:700">14.2 GB</span></div>
          <div class="row-between"><span class="t-mom">发音缓存</span><span class="t-mom" style="font-weight:700">3.1 GB</span></div>
        </div>
        <button class="btn btn-primary btn-block btn-lg">清理 30 天前的录音</button>
        <p class="t-mom-sm center-text" style="margin:0">可释放约 11.8 GB，收藏过的不会被删</p>
      </div>

      <!-- 7 屏幕时间用尽 -->
      <div class="card stack-4 center-text">
        <div class="big-ico" style="font-size:44px">🐙</div>
        <h3 class="t-zh-lg" style="margin:0">今天就到这儿啦</h3>
        <p class="t-mom" style="margin:0">章鱼老师说：明天再来找我玩。</p>
        <div class="card stack-3" style="box-shadow:none">
          <div class="row" style="gap:var(--sp-6);justify-content:center">
            <span style="text-align:center"><div style="font-size:30px;font-weight:800">5</div><div class="t-mom-sm">个新词</div></span>
            <span style="text-align:center"><div style="font-size:30px;font-weight:800">7</div><div class="t-mom-sm">次跟读</div></span>
          </div>
        </div>
        <p class="note" style="margin:0">PRD 5.4 / 11.3：到点是<b>柔性收尾</b>而非硬弹窗——走小结页、吉祥物道别，母亲仍可查看日报。</p>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref } from 'vue'

const locationHref = ref(typeof location !== 'undefined' ? location.href : 'http://192.168.1.20:8080')
</script>
