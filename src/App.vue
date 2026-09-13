<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import ConfigPage from './components/ConfigPage.vue'

const wallpaperBase64 = ref<string | null>(null)

// 获取当前桌面壁纸
async function fetchCurrentWallpaper() {
  try {
    const result = await invoke<string | null>('get_current_wallpaper')
    if (result) {
      wallpaperBase64.value = result
    }
  } catch (error) {
    console.error('获取当前壁纸失败:', error)
  }
}

let unlistenFocus: (() => void) | null = null

onMounted(async () => {
  // 初始加载壁纸
  await fetchCurrentWallpaper()

  // 监听窗口焦点事件
  const appWindow = getCurrentWindow()
  unlistenFocus = await appWindow.onFocusChanged(({ payload: focused }: { payload: boolean }) => {
    if (focused) {
      fetchCurrentWallpaper()
    }
  })
})

onUnmounted(() => {
  if (unlistenFocus) {
    unlistenFocus()
  }
})
</script>

<template>
  <div class="app-wrapper">
    <!-- 壁纸背景层 -->
    <div
      v-if="wallpaperBase64"
      class="wallpaper-background"
      :style="{ backgroundImage: `url(${wallpaperBase64})` }"
    ></div>
    <!-- 渐变背景层（备用） -->
    <div v-else class="gradient-background"></div>
    <!-- 内容层 -->
    <div class="content-layer">
      <ConfigPage />
    </div>
  </div>
</template>

<style>
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  min-height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
}

.app-wrapper {
  width: 100%;
  min-height: 100vh;
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
}

.wallpaper-background {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-size: cover;
  background-position: center;
  background-repeat: no-repeat;
  filter: blur(8px);
  transform: scale(1.05); /* 防止模糊边缘露出 */
  z-index: -2;
}

.wallpaper-background::after {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.4); /* 添加暗色遮罩提高可读性 */
}

.gradient-background {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  z-index: -2;
}

.content-layer {
  width: 100%;
  max-width: 800px;
  margin: 20px;
  position: relative;
  z-index: 1;
}

/* 强制浅色模式 */
:root[data-theme="light"] .gradient-background {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
}

/* 强制深色模式 */
:root[data-theme="dark"] .gradient-background {
  background: linear-gradient(135deg, #1a1a2e 0%, #16213e 100%);
}

:root[data-theme="dark"] .wallpaper-background::after {
  background: rgba(0, 0, 0, 0.5); /* 深色模式下更深的遮罩 */
}

/* 随系统 - 深色模式 */
@media (prefers-color-scheme: dark) {
  .gradient-background {
    background: linear-gradient(135deg, #1a1a2e 0%, #16213e 100%);
  }

  .wallpaper-background::after {
    background: rgba(0, 0, 0, 0.5);
  }
}

/* 强制浅色模式下覆盖系统深色 */
@media (prefers-color-scheme: dark) {
  :root[data-theme="light"] .gradient-background {
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  }

  :root[data-theme="light"] .wallpaper-background::after {
    background: rgba(0, 0, 0, 0.3);
  }
}
</style>
