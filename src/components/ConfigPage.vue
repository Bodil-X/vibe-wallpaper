<template>
  <div class="config-container">
    <div class="header">
      <h1>Vibe 壁纸设置</h1>
      <button class="theme-toggle" @click="cycleTheme" :title="'当前: ' + themeLabels[config.theme]">
        <span class="theme-icon">{{ themeIcons[config.theme] }}</span>
        <span class="theme-label">{{ themeLabels[config.theme] }}</span>
      </button>
    </div>

    <!-- 刷新间隔设置 -->
    <div class="config-section">
      <label>自动刷新间隔</label>
      <select v-model="config.refresh_interval" @change="saveConfig">
        <option value="5min">5分钟</option>
        <option value="10min">10分钟</option>
        <option value="15min">15分钟</option>
        <option value="30min">30分钟</option>
        <option value="1hour">1小时</option>
        <option value="2hour">2小时</option>
        <option value="4hour">4小时</option>
        <option value="6hour">6小时</option>
        <option value="12hour">12小时</option>
        <option value="1day">1天</option>
      </select>
    </div>

    <!-- 分辨率设置 -->
    <div class="config-section">
      <label>壁纸分辨率</label>
      <select v-model="config.resolution" @change="saveConfig">
        <option value="auto">自动检测</option>
        <option value="1920x1080">1920×1080</option>
        <option value="2560x1440">2560×1440</option>
        <option value="3840x2160">3840×2160</option>
        <option value="5120x2880">5120×2880</option>
      </select>
    </div>

    <!-- 多显示器设置 -->
    <div class="config-section">
      <label class="checkbox-label">
        <input type="checkbox" v-model="config.different_per_monitor" @change="saveConfig">
        为不同显示器设置不同壁纸 (Windows 8+)
      </label>
    </div>

    <!-- 随机自启动 -->
    <div class="config-section">
      <label class="checkbox-label">
        <input type="checkbox" v-model="config.auto_start" @change="toggleAutoStart">
        开机自动启动
      </label>
    </div>

    <!-- 保存位置 -->
    <div class="config-section">
      <label>壁纸保存位置</label>
      <div class="path-input-group">
        <input type="text" v-model="config.save_location" readonly>
        <button @click="selectSaveLocation">选择文件夹</button>
        <button @click="setDefaultLocation">默认</button>
      </div>
    </div>

    <!-- 代理配置 -->
    <div class="config-section">
      <label class="checkbox-label">
        <input type="checkbox" v-model="config.proxy_enabled" @change="toggleProxy">
        启用代理
      </label>
      <div v-if="config.proxy_enabled" class="proxy-settings">
        <input type="text" v-model="config.proxy_url" placeholder="代理服务器地址" @change="saveConfig">
        <input type="text" v-model="config.proxy_username" placeholder="用户名 (可选)" @change="saveConfig">
        <input type="password" v-model="config.proxy_password" placeholder="密码 (可选)" @change="saveConfig">
      </div>
    </div>

    <!-- 壁纸源 URL -->
    <div class="config-section">
      <label>壁纸源</label>
      <select v-model="config.source_type" @change="onSourceTypeChange">
        <option value="unsplash">Unsplash</option>
        <option value="custom">自定义</option>
      </select>
      <div v-if="config.source_type === 'custom'" class="custom-source-settings">
        <input
          type="text"
          v-model="config.source_url"
          @change="saveConfig"
          placeholder="https://example.com/random/{{w}}x{{h}}"
        >
        <p class="hint">
          使用 <code v-pre>{{w}}</code> 和 <code v-pre>{{h}}</code> 作为宽高占位符。
          若 URL 中不含这两个占位符，则不会自动添加分辨率参数。
        </p>
      </div>
    </div>

    <!-- 操作按钮 -->
    <div class="action-buttons">
      <button @click="refreshNow" class="primary-button">立即刷新</button>
      <button @click="openPreviewWindow" class="secondary-button">获取新壁纸</button>
      <button @click="saveCurrentWallpaper">保存当前壁纸</button>
      <button @click="hideToTray">最小化到托盘</button>
    </div>

    <!-- 状态显示 -->
    <div class="status-bar" v-if="statusMessage">
      {{ statusMessage }}
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { appDataDir } from '@tauri-apps/api/path'
import { getCurrentWindow } from '@tauri-apps/api/window'

interface Config {
  refresh_interval: string
  resolution: string
  different_per_monitor: boolean
  auto_start: boolean
  save_location: string
  proxy_enabled: boolean
  proxy_url: string
  proxy_username: string
  proxy_password: string
  source_type: 'unsplash' | 'custom'
  source_url: string
  theme: 'light' | 'dark' | 'system'
}

const config = reactive<Config>({
  refresh_interval: '1hour',
  resolution: 'auto',
  different_per_monitor: true,
  auto_start: false,
  save_location: '',
  proxy_enabled: false,
  proxy_url: '',
  proxy_username: '',
  proxy_password: '',
  source_type: 'unsplash',
  source_url: 'https://source.unsplash.com/random/{{w}}x{{h}}',
  theme: 'system'
})

const statusMessage = ref('')

// 主题图标映射
const themeIcons = {
  light: '☀️',
  dark: '🌙',
  system: '💻'
}

// 主题标签映射
const themeLabels = {
  light: '白天',
  dark: '黑夜',
  system: '随系统'
}

// 应用主题到 DOM
function applyTheme(theme: 'light' | 'dark' | 'system') {
  const root = document.documentElement
  if (theme === 'system') {
    root.removeAttribute('data-theme')
  } else {
    root.setAttribute('data-theme', theme)
  }
}

// 切换主题
function cycleTheme() {
  const themes: Array<'light' | 'dark' | 'system'> = ['light', 'dark', 'system']
  const currentIndex = themes.indexOf(config.theme)
  const nextIndex = (currentIndex + 1) % themes.length
  config.theme = themes[nextIndex]
  applyTheme(config.theme)
  saveConfig()
}

// 加载配置
async function loadConfig() {
  try {
    const savedConfig = await invoke('load_config')
    Object.assign(config, savedConfig)
    // 应用保存的主题
    applyTheme(config.theme)
  } catch (error) {
    console.error('加载配置失败:', error)
    // 使用默认配置
    config.save_location = await appDataDir() + 'wallpapers'
  }
}

// 保存配置
async function saveConfig() {
  try {
    console.log('保存配置:', JSON.stringify(config, null, 2))
    await invoke('save_config', { config })
    showStatus('配置已保存')
    // 更新定时器
    await invoke('update_scheduler')
    console.log('配置保存成功，定时器已更新')
  } catch (error) {
    console.error('保存配置失败:', error)
    showStatus('保存配置失败', 'error')
  }
}

// 切换自启动
async function toggleAutoStart() {
  try {
    await invoke('toggle_auto_start', { enabled: config.auto_start })
    await saveConfig()
    showStatus(config.auto_start ? '已启用开机自启动' : '已禁用开机自启动')
  } catch (error) {
    console.error('设置自启动失败:', error)
    config.auto_start = !config.auto_start // 恢复原状态
    showStatus('设置自启动失败', 'error')
  }
}

// 选择保存位置
async function selectSaveLocation() {
  try {
    const selected = await open({
      directory: true,
      multiple: false,
      defaultPath: config.save_location
    })
    if (selected && !Array.isArray(selected)) {
      config.save_location = selected
      await saveConfig()
    }
  } catch (error) {
    console.error('选择文件夹失败:', error)
  }
}

// 设置默认位置
async function setDefaultLocation() {
  config.save_location = await appDataDir() + 'wallpapers'
  await saveConfig()
}

// 切换代理
async function toggleProxy() {
  await saveConfig()
  showStatus(config.proxy_enabled ? '代理已启用' : '代理已禁用')
}

// 切换壁纸源类型
async function onSourceTypeChange() {
  if (config.source_type === 'unsplash') {
    // 使用 Unsplash 默认地址
    config.source_url = 'https://source.unsplash.com/random/{{w}}x{{h}}'
  }
  await saveConfig()
}

// 立即刷新
function refreshNow() {
  try {
    showStatus('正在获取壁纸...')
    console.log('手动刷新壁纸，当前配置:', JSON.stringify(config, null, 2))
    invoke('refresh_wallpaper')
      .then(() => {
        showStatus('壁纸已更新')
        console.log('壁纸刷新完成')
      })
      .catch((error) => {
        console.error('刷新壁纸失败:', error)
        showStatus('刷新壁纸失败', 'error')
      })
  } catch (error) {
    console.error('刷新壁纸失败:', error)
    showStatus('刷新壁纸失败', 'error')
  }
}

// 打开预览窗口
async function openPreviewWindow() {
  try {
    showStatus('正在打开壁纸预览窗口...')
    await invoke('open_preview_window')
    showStatus('壁纸预览窗口已打开')
  } catch (error) {
    console.error('打开预览窗口失败:', error)
    showStatus('打开预览窗口失败', 'error')
  }
}

// 保存当前壁纸
async function saveCurrentWallpaper() {
  try {
    showStatus('正在保存壁纸...')
    await invoke('save_current_wallpaper')
    showStatus('壁纸已保存')
  } catch (error) {
    console.error('保存壁纸失败:', error)
    showStatus('保存壁纸失败', 'error')
  }
}

// 显示状态消息
function showStatus(message: string, _type: 'success' | 'error' | 'info' = 'info') {
  statusMessage.value = message
  setTimeout(() => {
    statusMessage.value = ''
  }, 3000)
}

// 最小化到托盘
async function hideToTray() {
  try {
    const window = getCurrentWindow()
    await window.hide()
  } catch (error) {
    console.error('隐藏窗口失败:', error)
  }
}

onMounted(() => {
  loadConfig()
})
</script>

<style scoped>
.config-container {
  max-width: 600px;
  margin: 0 auto;
  padding: 20px;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
}

.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 30px;
}

h1 {
  text-align: center;
  color: #fff;
  margin: 0;
  font-size: 24px;
  font-weight: 600;
  text-shadow: 0 2px 4px rgba(0, 0, 0, 0.3);
}

.theme-toggle {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 12px;
  border: 1px solid #ced4da;
  border-radius: 20px;
  background: white;
  color: #495057;
  font-size: 13px;
  cursor: pointer;
  transition: all 0.2s;
}

.theme-toggle:hover {
  background: #f8f9fa;
  border-color: #adb5bd;
}

.theme-icon {
  font-size: 16px;
}

.theme-label {
  font-weight: 500;
}

.config-section {
  margin-bottom: 10px;
  padding: 15px;
  background: #f8f9fa;
  border-radius: 8px;
  border: 1px solid #e9ecef;
}

.config-section label {
  display: block;
  margin-bottom: 8px;
  font-weight: 500;
  color: #495057;
}

.config-section select,
.config-section input[type="text"],
.config-section input[type="password"] {
  width: 100%;
  padding: 8px 12px;
  border: 1px solid #ced4da;
  border-radius: 4px;
  font-size: 14px;
  box-sizing: border-box;
}

.config-section input[type="text"][readonly] {
  background-color: #e9ecef;
}

.checkbox-label {
  display: flex !important;
  align-items: center;
  cursor: pointer;
  margin-bottom: 0 !important;
}

.checkbox-label input[type="checkbox"] {
  width: auto;
  margin-right: 8px;
  margin-bottom: 0;
}

.path-input-group {
  display: flex;
  gap: 8px;
}

.path-input-group input {
  flex: 1;
}

.path-input-group button {
  padding: 8px 16px;
  white-space: nowrap;
}

.proxy-settings {
  margin-top: 10px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.custom-source-settings {
  margin-top: 10px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.custom-source-settings .hint {
  margin: 0;
  font-size: 12px;
  color: #6c757d;
  line-height: 1.4;
}

.custom-source-settings .hint code {
  background: #e9ecef;
  padding: 2px 6px;
  border-radius: 3px;
  font-family: monospace;
  font-size: 11px;
}

.action-buttons {
  display: flex;
  gap: 12px;
  justify-content: center;
  margin-top: 30px;
  padding-top: 20px;
  border-top: 1px solid #e9ecef;
}

button {
  padding: 10px 20px;
  border: 1px solid #ced4da;
  border-radius: 6px;
  background: white;
  color: #495057;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
}

button:hover {
  background: #f8f9fa;
  border-color: #adb5bd;
}

.primary-button {
  background: #007bff;
  color: white;
  border-color: #007bff;
}

.primary-button:hover {
  background: #0056b3;
  border-color: #0056b3;
}

.secondary-button {
  background: #6c757d;
  color: white;
  border-color: #6c757d;
}

.secondary-button:hover {
  background: #545b62;
  border-color: #545b62;
}

/* 加载状态样式 */
.loading-state {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(255, 255, 255, 0.95);
  z-index: 100;
}

.loading-overlay {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 40px;
  color: #6c757d;
  background: white;
  border-radius: 8px;
  box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
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

/* 模态框样式 */
.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background: rgba(0, 0, 0, 0.7);
  display: flex;
  justify-content: center;
  align-items: center;
  z-index: 2000;
}

.modal-content {
  background: white;
  border-radius: 12px;
  width: 98%;
  max-width: 1400px;
  max-height: 98vh;
  display: flex;
  flex-direction: column;
  box-shadow: 0 10px 40px rgba(0, 0, 0, 0.3);
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 20px;
  border-bottom: 1px solid #e9ecef;
}

.modal-header h2 {
  margin: 0;
  font-size: 20px;
  color: #333;
}

.close-button {
  background: none;
  border: none;
  font-size: 28px;
  cursor: pointer;
  color: #999;
  padding: 0;
  width: 30px;
  height: 30px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.close-button:hover {
  color: #333;
  background: none;
}

.modal-body {
  position: relative;
  padding: 20px;
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: auto;
  min-height: 400px;
}

.image-preview {
  flex: 1;
  display: flex;
  justify-content: center;
  align-items: center;
  margin-bottom: 20px;
  min-height: 600px;
  max-height: 70vh;
  background: #f8f9fa;
  border-radius: 8px;
  overflow: hidden;
}

.image-preview img {
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
}

.loading-placeholder {
  color: #6c757d;
  font-size: 18px;
}

.image-navigation {
  display: flex;
  justify-content: center;
  align-items: center;
  gap: 20px;
  margin-bottom: 20px;
}

.image-navigation button {
  padding: 8px 16px;
  min-width: 80px;
}

.image-navigation button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.image-counter {
  font-weight: 500;
  color: #495057;
}

.modal-actions {
  display: flex;
  flex-direction: column;
  gap: 15px;
  padding-top: 20px;
  border-top: 1px solid #e9ecef;
}

.monitor-selection {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.monitor-selection label {
  font-weight: 500;
  color: #495057;
}

.monitor-selection select {
  padding: 8px 12px;
  border: 1px solid #ced4da;
  border-radius: 4px;
  font-size: 14px;
}

.action-buttons-group {
  display: flex;
  gap: 12px;
  justify-content: center;
}

.action-buttons-group button {
  flex: 1;
  max-width: 200px;
}

.status-bar {
  position: fixed;
  bottom: 20px;
  left: 50%;
  transform: translateX(-50%);
  padding: 12px 24px;
  background: #28a745;
  color: white;
  border-radius: 6px;
  font-weight: 500;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
  z-index: 1000;
}

@media (prefers-color-scheme: dark) {
  .config-container {
    background: #1a1a1a;
    color: #e0e0e0;
  }

  h1 {
    color: #fff;
    text-shadow: 0 2px 4px rgba(0, 0, 0, 0.5);
  }

  .theme-toggle {
    background: #2d2d2d;
    color: #e0e0e0;
    border-color: #404040;
  }

  .theme-toggle:hover {
    background: #404040;
  }

  .config-section {
    background: #2d2d2d;
    border-color: #404040;
  }

  .config-section label {
    color: #e0e0e0;
  }

  .config-section select,
  .config-section input {
    background: #1a1a1a;
    color: #e0e0e0;
    border-color: #404040;
  }

  .config-section input[type="text"][readonly] {
    background: #2d2d2d;
    color: #e0e0e0;
  }

  button {
    background: #2d2d2d;
    color: #e0e0e0;
    border-color: #404040;
  }

  button:hover {
    background: #404040;
  }

  .primary-button {
    background: #0066cc;
    border-color: #0066cc;
  }

  .primary-button:hover {
    background: #0052a3;
    border-color: #0052a3;
  }

  /* 深色模式模态框样式 */
  .modal-overlay {
    background: rgba(0, 0, 0, 0.8);
  }

  .modal-content {
    background: #2d2d2d;
    border: 1px solid #404040;
  }

  .modal-header {
    border-bottom-color: #404040;
  }

  .modal-header h2 {
    color: #e0e0e0;
  }

  .close-button {
    color: #aaa;
  }

  .close-button:hover {
    color: #e0e0e0;
    background: none;
  }

  .image-preview {
    background: #1a1a1a;
  }

  .loading-placeholder {
    color: #aaa;
  }

  .image-counter {
    color: #e0e0e0;
  }

  .modal-actions {
    border-top-color: #404040;
  }

  .monitor-selection label {
    color: #e0e0e0;
  }

  .monitor-selection select {
    background: #1a1a1a;
    color: #e0e0e0;
    border-color: #404040;
  }

  .custom-source-settings .hint {
    color: #aaa;
  }

  .custom-source-settings .hint code {
    background: #404040;
    color: #e0e0e0;
  }
}
</style>

<!-- 强制主题模式样式 (非 scoped) -->
<style>
/* 强制深色模式 */
:root[data-theme="dark"] .config-container {
  background: #1a1a1a;
  color: #e0e0e0;
}

:root[data-theme="dark"] .config-container h1 {
  color: #fff;
  text-shadow: 0 2px 4px rgba(0, 0, 0, 0.5);
}

:root[data-theme="dark"] .config-container .theme-toggle {
  background: #2d2d2d;
  color: #e0e0e0;
  border-color: #404040;
}

:root[data-theme="dark"] .config-container .theme-toggle:hover {
  background: #404040;
}

:root[data-theme="dark"] .config-container .config-section {
  background: #2d2d2d;
  border-color: #404040;
}

:root[data-theme="dark"] .config-container .config-section label {
  color: #e0e0e0;
}

:root[data-theme="dark"] .config-container .config-section select,
:root[data-theme="dark"] .config-container .config-section input {
  background: #1a1a1a;
  color: #e0e0e0;
  border-color: #404040;
}

:root[data-theme="dark"] .config-container .config-section input[type="text"][readonly] {
  background: #2d2d2d;
  color: #e0e0e0;
}

:root[data-theme="dark"] .config-container button {
  background: #2d2d2d;
  color: #e0e0e0;
  border-color: #404040;
}

:root[data-theme="dark"] .config-container button:hover {
  background: #404040;
}

:root[data-theme="dark"] .config-container .primary-button {
  background: #0066cc;
  border-color: #0066cc;
}

:root[data-theme="dark"] .config-container .primary-button:hover {
  background: #0052a3;
  border-color: #0052a3;
}

:root[data-theme="dark"] .config-container .secondary-button {
  background: #545b62;
  border-color: #545b62;
}

/* 强制浅色模式 - 覆盖系统深色偏好 */
@media (prefers-color-scheme: dark) {
  :root[data-theme="light"] .config-container {
    background: transparent;
    color: inherit;
  }

  :root[data-theme="light"] .config-container h1 {
    color: #fff;
    text-shadow: 0 2px 4px rgba(0, 0, 0, 0.3);
  }

  :root[data-theme="light"] .config-container .theme-toggle {
    background: white;
    color: #495057;
    border-color: #ced4da;
  }

  :root[data-theme="light"] .config-container .theme-toggle:hover {
    background: #f8f9fa;
  }

  :root[data-theme="light"] .config-container .config-section {
    background: #f8f9fa;
    border-color: #e9ecef;
  }

  :root[data-theme="light"] .config-container .config-section label {
    color: #495057;
  }

  :root[data-theme="light"] .config-container .config-section select,
  :root[data-theme="light"] .config-container .config-section input {
    background: white;
    color: inherit;
    border-color: #ced4da;
  }

  :root[data-theme="light"] .config-container .config-section input[type="text"][readonly] {
    background: #e9ecef;
    color: #495057;
  }

  :root[data-theme="light"] .config-container button {
    background: white;
    color: #495057;
    border-color: #ced4da;
  }

  :root[data-theme="light"] .config-container button:hover {
    background: #f8f9fa;
  }

  :root[data-theme="light"] .config-container .primary-button {
    background: #007bff;
    color: white;
    border-color: #007bff;
  }

  :root[data-theme="light"] .config-container .primary-button:hover {
    background: #0056b3;
    border-color: #0056b3;
  }

  :root[data-theme="light"] .config-container .secondary-button {
    background: #6c757d;
    color: white;
    border-color: #6c757d;
  }
}
</style>