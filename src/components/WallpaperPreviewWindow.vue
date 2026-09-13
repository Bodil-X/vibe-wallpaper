<template>
  <div class="preview-window">
    <div class="preview-content">
      <!-- 加载状态 - 下载新图时 -->
      <div v-if="isDownloading" class="loading-overlay">
        <div class="loading-wrapper" :style="{ '--progress': downloadProgress }">
          <div class="loading-content">
            <div class="spinner"></div>
            <p>正在下载壁纸... {{ downloadProgress }}%</p>
          </div>
        </div>
      </div>

      <div v-if="previewImages.length === 0 && !isDownloading" class="empty-state">
        <p>正在获取壁纸...</p>
      </div>

      <div v-else-if="currentPreview" class="image-container">
        <img :src="currentPreview.base64" :alt="`壁纸 ${currentIndex + 1}`" @error="handleImageError">
      </div>

      <div class="preview-controls">
        <div class="navigation-controls">
          <button @click="previousImage" :disabled="currentIndex === 0">上一张</button>
          <span class="counter">{{ currentIndex + 1 }} / {{ previewImages.length }}</span>
          <button @click="nextImage">{{ currentIndex === previewImages.length - 1 ? '下一张（新图）' : '下一张' }}</button>
        </div>

        <div class="action-controls">
          <div class="monitor-selection" v-if="differentPerMonitor">
            <label>选择显示器：</label>
            <select v-model="selectedMonitor">
              <option value="">应用到所有显示器</option>
              <option v-for="i in monitorCount" :key="i" :value="(i - 1).toString()">显示器 {{ i }}</option>
            </select>
          </div>

          <div class="button-group">
            <button @click="setAsWallpaper" class="primary-button">设为壁纸</button>
            <button @click="saveThisWallpaper" class="secondary-button">保存到本地</button>
          </div>
        </div>
      </div>
    </div>

    <!-- Toast 提示 -->
    <Transition name="toast">
      <div v-if="toast.show" :class="['toast', toast.type]">
        {{ toast.message }}
      </div>
    </Transition>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, reactive } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

interface WallpaperPreview {
  id: string
  url: string
  path: string
  base64?: string
}

const previewImages = ref<WallpaperPreview[]>([])
const currentIndex = ref(0)
const selectedMonitor = ref('')
const isDownloading = ref(false)
const differentPerMonitor = ref(false)
const monitorCount = ref(1)
const downloadProgress = ref(0)

// Toast 状态
const toast = reactive({
  show: false,
  message: '',
  type: 'success' as 'success' | 'error'
})

let toastTimer: number | null = null

// 显示 Toast 提示
function showToast(message: string, type: 'success' | 'error' = 'success') {
  if (toastTimer) {
    clearTimeout(toastTimer)
  }
  toast.message = message
  toast.type = type
  toast.show = true
  toastTimer = window.setTimeout(() => {
    toast.show = false
  }, 3000)
}

const currentPreview = computed(() => previewImages.value[currentIndex.value])

let unlistenProgress: (() => void) | null = null
let lastLoggedProgress = 0

// 获取配置
async function loadConfig() {
  try {
    const config = await invoke<any>('load_config')
    differentPerMonitor.value = config.different_per_monitor
    // 应用主题设置
    applyTheme(config.theme || 'system')
  } catch (error) {
    console.error('加载配置失败:', error)
  }
}

// 应用主题到 DOM
function applyTheme(theme: string) {
  const root = document.documentElement
  if (theme === 'system') {
    root.removeAttribute('data-theme')
  } else {
    root.setAttribute('data-theme', theme)
  }
}

// 获取显示器数量
async function loadMonitorCount() {
  try {
    const count = await invoke<number>('get_monitor_count')
    monitorCount.value = count
    console.log(`检测到 ${count} 个显示器`)
  } catch (error) {
    console.error('获取显示器数量失败:', error)
    // 默认至少有1个显示器
    monitorCount.value = 1
  }
}

// 从后端加载已保存的预览图片
async function loadSavedPreviews() {
  try {
    const savedImages = await invoke<WallpaperPreview[]>('get_preview_images')
    if (savedImages && savedImages.length > 0) {
      previewImages.value = savedImages
      currentIndex.value = 0
      console.log(`从后端加载了 ${savedImages.length} 张已保存的壁纸`)
      return true
    }
    return false
  } catch (error) {
    console.error('加载已保存的壁纸失败:', error)
    return false
  }
}

// 获取新壁纸
async function getNewWallpaper() {
  try {
    isDownloading.value = true
    downloadProgress.value = 0
    lastLoggedProgress = 0
    const preview = await invoke<WallpaperPreview>('fetch_single_wallpaper', { index: previewImages.value.length })

    previewImages.value = [...previewImages.value, preview]
    currentIndex.value = previewImages.value.length - 1
    console.log(`新壁纸已下载并保存，当前共有 ${previewImages.value.length} 张壁纸`)

    // 等待一小段时间，让用户看到 100% 的进度
    await new Promise(resolve => setTimeout(resolve, 500))
  } catch (error) {
    console.error('获取壁纸失败:', error)
    showToast('获取壁纸失败: ' + error, 'error')
  } finally {
    isDownloading.value = false
    // 延迟后再重置进度，确保用户能看到完整的进度条
    setTimeout(() => {
      downloadProgress.value = 0
    }, 100)
  }
}

// 上一张
function previousImage() {
  if (currentIndex.value > 0) {
    currentIndex.value--
  }
}

// 下一张
async function nextImage() {
  if (currentIndex.value < previewImages.value.length - 1) {
    currentIndex.value++
  } else {
    await getNewWallpaper()
  }
}

// 设为壁纸
async function setAsWallpaper() {
  if (!currentPreview.value) return

  try {
    await invoke('set_wallpaper_from_path', {
      imagePath: currentPreview.value.path,
      monitor: selectedMonitor.value ? parseInt(selectedMonitor.value) : null
    })
    showToast('壁纸已设置')
  } catch (error) {
    console.error('设置壁纸失败:', error)
    showToast('设置壁纸失败: ' + error, 'error')
  }
}

// 保存此壁纸
async function saveThisWallpaper() {
  if (!currentPreview.value) return

  try {
    await invoke('save_wallpaper_from_path', { imagePath: currentPreview.value.path })
    showToast('壁纸已保存到配置的保存位置')
  } catch (error) {
    console.error('保存壁纸失败:', error)
    showToast('保存壁纸失败: ' + error, 'error')
  }
}

// 处理图片加载错误
function handleImageError(event: Event) {
  console.error('图片加载失败:', event)
}

onMounted(async () => {
  await loadConfig()
  await loadMonitorCount()

  // 监听下载进度事件
  unlistenProgress = await listen<number>('download-progress', (event: { payload: number }) => {
    downloadProgress.value = event.payload
    // 每10%打印一次进度
    const currentTen = Math.floor(event.payload / 10) * 10
    if (currentTen > lastLoggedProgress || event.payload === 100) {
      console.log('下载进度:', event.payload + '%')
      lastLoggedProgress = currentTen
    }
  })

  // 首先尝试从后端加载已保存的预览图片
  const hasSavedImages = await loadSavedPreviews()

  // 如果没有已保存的图片，再获取新壁纸
  if (!hasSavedImages) {
    getNewWallpaper()
  }
})

onUnmounted(() => {
  // 清理事件监听
  if (unlistenProgress) {
    unlistenProgress()
  }
})
</script>

<style scoped>
/* 全局样式，防止滚动条 */
:global(html), :global(body) {
  margin: 0;
  padding: 0;
  overflow: hidden;
  width: 100%;
  height: 100%;
}

.preview-window {
  width: 100vw;
  height: 100vh;
  display: flex;
  flex-direction: column;
  background: #f5f5f5;
  overflow: hidden;
  box-sizing: border-box;
}

.preview-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 15px 30px;
  background: white;
  border-bottom: 1px solid #e0e0e0;
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
  flex-shrink: 0; /* 防止header被压缩 */
  box-sizing: border-box;
  width: 100%;
}

.preview-header h1 {
  margin: 0;
  font-size: 24px;
  color: #333;
}

.preview-content {
  flex: 1;
  position: relative;
  padding: 10px;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  box-sizing: border-box;
  width: 100%;
  height: 100vh;
}

.loading-overlay {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(255, 255, 255, 0.95);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}

.loading-wrapper {
  position: relative;
  padding: 6px;
  border-radius: 12px;
  background: #e0e0e0;
}

.loading-wrapper::before {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  border-radius: 12px;
  /* 顺时针进度：从顶部开始 */
  background: conic-gradient(
    from -90deg,
    #ff0000 0%,
    #ff7f00 calc(var(--progress, 0) * 0.1428%),
    #ffff00 calc(var(--progress, 0) * 0.2856%),
    #00ff00 calc(var(--progress, 0) * 0.4284%),
    #0000ff calc(var(--progress, 0) * 0.5712%),
    #4b0082 calc(var(--progress, 0) * 0.714%),
    #9400d3 calc(var(--progress, 0) * 0.8568%),
    #ff0000 calc(var(--progress, 0) * 1%),
    transparent calc(var(--progress, 0) * 1%),
    transparent 100%
  );
  /* 使用 mask 只显示边框部分 */
  -webkit-mask:
    linear-gradient(#fff 0 0) content-box,
    linear-gradient(#fff 0 0);
  -webkit-mask-composite: xor;
  mask:
    linear-gradient(#fff 0 0) content-box,
    linear-gradient(#fff 0 0);
  mask-composite: exclude;
  padding: 6px;
  animation: rainbow-rotate 3s linear infinite;
  box-shadow:
    0 0 20px rgba(255, 0, 0, 0.4),
    0 0 40px rgba(0, 255, 0, 0.3),
    0 0 60px rgba(0, 0, 255, 0.3);
}

@keyframes rainbow-rotate {
  0% {
    filter: hue-rotate(0deg);
  }
  100% {
    filter: hue-rotate(360deg);
  }
}

.loading-content {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 40px 60px;
  background: white;
  border-radius: 8px;
  min-width: 200px;
  z-index: 1;
}

.loading-content p {
  margin: 0;
  color: #6c757d;
  font-size: 16px;
}

.spinner {
  width: 50px;
  height: 50px;
  border: 5px solid #f3f3f3;
  border-top: 5px solid #007bff;
  border-radius: 50%;
  animation: spin 1s linear infinite;
  margin-bottom: 20px;
}

@keyframes spin {
  0% { transform: rotate(0deg); }
  100% { transform: rotate(360deg); }
}

.empty-state {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #6c757d;
  font-size: 18px;
}

.image-container {
  flex: 1;
  display: flex;
  justify-content: center;
  align-items: center;
  width: 100%;
  height: calc(100% - 150px);
  min-height: 300px;
  max-height: none;
  background: #f8f9fa;
  border-radius: 8px;
  margin-bottom: 15px;
  padding-top: 10px;
  padding-bottom: 10px;
  overflow: hidden;
  box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.1);
  box-sizing: border-box;
}

.image-container img {
  max-width: 100%;
  max-height: 100%;
  width: auto;
  height: auto;
  object-fit: contain; /* 保持宽高比 */
}

.preview-controls {
  display: flex;
  flex-direction: column;
  gap: 20px;
  flex-shrink: 0; /* 防止被压缩 */
  box-sizing: border-box;
  width: 100%;
}

.navigation-controls {
  display: flex;
  justify-content: center;
  align-items: center;
  gap: 20px;
}

.navigation-controls button {
  padding: 10px 20px;
  background: white;
  border: 1px solid #ddd;
  border-radius: 6px;
  cursor: pointer;
  font-size: 14px;
}

.navigation-controls button:hover:not(:disabled) {
  background: #f0f0f0;
}

.navigation-controls button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.counter {
  font-size: 16px;
  color: #666;
  font-weight: 500;
}

.action-controls {
  display: flex;
  flex-direction: column;
  gap: 15px;
  align-items: center;
}

.monitor-selection {
  display: flex;
  align-items: center;
  gap: 10px;
}

.monitor-selection label {
  font-size: 14px;
  color: #666;
}

.monitor-selection select {
  padding: 8px 12px;
  border: 1px solid #ddd;
  border-radius: 4px;
  font-size: 14px;
}

.button-group {
  display: flex;
  gap: 15px;
}

.secondary-button {
  padding: 10px 20px;
  background: #6c757d;
  color: white;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  font-size: 14px;
}

.secondary-button:hover {
  background: #545b62;
}

/* Toast 提示样式 */
.toast {
  position: fixed;
  bottom: 30px;
  left: 50%;
  transform: translateX(-50%);
  padding: 12px 24px;
  border-radius: 8px;
  font-size: 14px;
  font-weight: 500;
  color: white;
  z-index: 1000;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
}

.toast.success {
  background: #28a745;
}

.toast.error {
  background: #dc3545;
}

/* Toast 动画 */
.toast-enter-active,
.toast-leave-active {
  transition: all 0.3s ease;
}

.toast-enter-from,
.toast-leave-to {
  opacity: 0;
  transform: translateX(-50%) translateY(20px);
}

/* 深色模式适配 */
@media (prefers-color-scheme: dark) {
  .preview-window {
    background: #1a1a1a;
  }

  .preview-header {
    background: #2d2d2d;
    border-bottom-color: #404040;
  }

  .preview-header h1 {
    color: #e0e0e0;
  }

  .loading-overlay {
    background: rgba(26, 26, 26, 0.95);
  }

  .loading-wrapper {
    background: #404040;
  }

  .loading-wrapper::before {
    box-shadow:
      0 0 25px rgba(255, 0, 0, 0.5),
      0 0 50px rgba(0, 255, 0, 0.4),
      0 0 75px rgba(0, 0, 255, 0.4);
  }

  .loading-content {
    background: #2d2d2d;
  }

  .loading-content p {
    color: #e0e0e0;
  }

  .spinner {
    border-color: #404040;
    border-top-color: #0066cc;
  }

  .empty-state {
    color: #aaa;
  }

  .image-container {
    background: #2d2d2d;
    box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.3);
  }

  .navigation-controls button {
    background: #2d2d2d;
    border-color: #404040;
    color: #e0e0e0;
  }

  .navigation-controls button:hover:not(:disabled) {
    background: #404040;
  }

  .counter {
    color: #aaa;
  }

  .monitor-selection label {
    color: #aaa;
  }

  .monitor-selection select {
    background: #2d2d2d;
    border-color: #404040;
    color: #e0e0e0;
  }

  .primary-button {
    background: #0066cc;
    border-color: #0066cc;
  }

  .primary-button:hover {
    background: #0052a3;
  }

  .secondary-button {
    background: #545b62;
  }

  .secondary-button:hover {
    background: #404040;
  }
}
</style>

<!-- 强制主题模式样式 (非 scoped) -->
<style>
/* 强制深色模式 */
:root[data-theme="dark"] .preview-window {
  background: #1a1a1a;
}

:root[data-theme="dark"] .preview-header {
  background: #2d2d2d;
  border-bottom-color: #404040;
}

:root[data-theme="dark"] .preview-header h1 {
  color: #e0e0e0;
}

:root[data-theme="dark"] .loading-overlay {
  background: rgba(26, 26, 26, 0.95);
}

:root[data-theme="dark"] .loading-wrapper {
  background: #404040;
}

:root[data-theme="dark"] .loading-content {
  background: #2d2d2d;
}

:root[data-theme="dark"] .loading-content p {
  color: #e0e0e0;
}

:root[data-theme="dark"] .spinner {
  border-color: #404040;
  border-top-color: #0066cc;
}

:root[data-theme="dark"] .empty-state {
  color: #aaa;
}

:root[data-theme="dark"] .image-container {
  background: #2d2d2d;
  box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.3);
}

:root[data-theme="dark"] .navigation-controls button {
  background: #2d2d2d;
  border-color: #404040;
  color: #e0e0e0;
}

:root[data-theme="dark"] .navigation-controls button:hover:not(:disabled) {
  background: #404040;
}

:root[data-theme="dark"] .counter {
  color: #aaa;
}

:root[data-theme="dark"] .monitor-selection label {
  color: #aaa;
}

:root[data-theme="dark"] .monitor-selection select {
  background: #2d2d2d;
  border-color: #404040;
  color: #e0e0e0;
}

:root[data-theme="dark"] .primary-button {
  background: #0066cc !important;
  border-color: #0066cc !important;
}

:root[data-theme="dark"] .primary-button:hover {
  background: #0052a3 !important;
}

:root[data-theme="dark"] .secondary-button {
  background: #545b62;
}

:root[data-theme="dark"] .secondary-button:hover {
  background: #404040;
}

/* 强制浅色模式 - 覆盖系统深色偏好 */
@media (prefers-color-scheme: dark) {
  :root[data-theme="light"] .preview-window {
    background: #f5f5f5;
  }

  :root[data-theme="light"] .preview-header {
    background: white;
    border-bottom-color: #e0e0e0;
  }

  :root[data-theme="light"] .preview-header h1 {
    color: #333;
  }

  :root[data-theme="light"] .loading-overlay {
    background: rgba(255, 255, 255, 0.95);
  }

  :root[data-theme="light"] .loading-wrapper {
    background: #e0e0e0;
  }

  :root[data-theme="light"] .loading-content {
    background: white;
  }

  :root[data-theme="light"] .loading-content p {
    color: #6c757d;
  }

  :root[data-theme="light"] .spinner {
    border-color: #f3f3f3;
    border-top-color: #007bff;
  }

  :root[data-theme="light"] .empty-state {
    color: #6c757d;
  }

  :root[data-theme="light"] .image-container {
    background: #f8f9fa;
    box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.1);
  }

  :root[data-theme="light"] .navigation-controls button {
    background: white;
    border-color: #ddd;
    color: inherit;
  }

  :root[data-theme="light"] .navigation-controls button:hover:not(:disabled) {
    background: #f0f0f0;
  }

  :root[data-theme="light"] .counter {
    color: #666;
  }

  :root[data-theme="light"] .monitor-selection label {
    color: #666;
  }

  :root[data-theme="light"] .monitor-selection select {
    background: white;
    border-color: #ddd;
    color: inherit;
  }

  :root[data-theme="light"] .primary-button {
    background: #007bff !important;
    border-color: #007bff !important;
    color: white !important;
  }

  :root[data-theme="light"] .primary-button:hover {
    background: #0056b3 !important;
  }

  :root[data-theme="light"] .secondary-button {
    background: #6c757d;
    color: white;
  }

  :root[data-theme="light"] .secondary-button:hover {
    background: #545b62;
  }
}
</style>
