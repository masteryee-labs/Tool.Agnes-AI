// Subagents list for the sidebar
const subagents = [
  { id: 1, name: "WorkflowArchitectureOptimizer", role: "工作流拓撲架構師", status: "sleeping" },
  { id: 2, name: "WorkflowRuntimeEvaluator", role: "工作流運行評估員", status: "sleeping" },
  { id: 3, name: "SlopVibeAuditor", role: "AI 語意與氛圍稽核員", status: "sleeping" },
  { id: 4, name: "SlopPathPurgeSpecialist", role: "代碼殘渣與路徑清理專員", status: "sleeping" },
  { id: 5, name: "OrchestratorAgent", role: "主編排調度官", status: "sleeping" },
  { id: 6, name: "LocaleCalibrationSpecialist", role: "環境語系校準專家", status: "sleeping" },
  { id: 7, name: "LeadSystemArchitect", role: "跨平台系統首席架構師", status: "sleeping" },
  { id: 8, name: "PerformanceArchitectureEngineer", role: "極致效能架構師", status: "sleeping" },
  { id: 9, name: "PerformanceAnalyticsEngineer", role: "資源動態分析師", status: "sleeping" },
  { id: 10, name: "PerformanceEfficiencyReviewer", role: "低耗能與記憶體審查員", status: "sleeping" },
  { id: 11, name: "SecurityArchitectureDesigner", role: "安全架構設計師", status: "sleeping" },
  { id: 12, name: "DefensiveCodingSpecialist", role: "防禦性編程實作專家", status: "sleeping" },
  { id: 13, name: "SecurityComplianceAuditor", role: "合規與審查員", status: "sleeping" },
  { id: 14, name: "CoreEngineCoder", role: "核心引擎開發工程師", status: "sleeping" },
  { id: 15, name: "IntegrationEngineer", role: "跨平台服務整合工程師", status: "sleeping" },
  { id: 16, name: "MultimodalMediaSpecialist", role: "多模態媒體生成專家", status: "sleeping" },
  { id: 17, name: "SandboxRuntimeTester", role: "自動化虛擬沙盒測試員", status: "sleeping" }
];

let activeWorkspace = "C:\\Users\\MasterYee\\Documents\\Program\\Agnes-AI";
let activeProjectId = null;
let activeTaskId = null;
let currentAgentReplyEl = null;

let projects = [];
let selectedFoldersForNewProject = [];
let selectedFoldersForCurrentProject = [];
let activeSettingProjectId = null;

document.addEventListener("DOMContentLoaded", () => {
  initProjectManagement();
  initRightSidebar();
  initSettings();
  initSubagents();
  initWorkspace();
  initChatAgent();
  initCollapsibleTrace();
  initIDEButton();
});

// Right sidebar collapsible panel and tab switching
function initRightSidebar() {
  const toggleBtn = document.getElementById("btn-toggle-right-sidebar");
  const rightSidebar = document.getElementById("right-sidebar");
  
  if (toggleBtn && rightSidebar) {
    toggleBtn.addEventListener("click", () => {
      rightSidebar.classList.toggle("collapsed");
    });
  }

  const tabButtons = document.querySelectorAll(".right-sidebar-tab-btn");
  const tabPanes = document.querySelectorAll(".right-tab-pane");

  tabButtons.forEach(btn => {
    btn.addEventListener("click", () => {
      const target = btn.getAttribute("data-right-tab");
      
      tabButtons.forEach(b => b.classList.remove("active"));
      tabPanes.forEach(p => p.classList.remove("active"));
      
      btn.classList.add("active");
      const targetPane = document.getElementById(target);
      if (targetPane) {
        targetPane.classList.add("active");
      }
    });
  });
}

function initProjectManagement() {
  const btnNewChat = document.getElementById("btn-new-chat");
  const sidebarBtnHistory = document.getElementById("sidebar-btn-history");
  const sidebarBtnScheduled = document.getElementById("sidebar-btn-scheduled");
  const btnProjectPlus = document.getElementById("btn-project-plus");
  const projectPlusDropdown = document.getElementById("project-plus-dropdown");
  const menuNewProject = document.getElementById("menu-new-project");
  const menuQuickStart = document.getElementById("menu-quick-start");
  const btnSettingsFooter = document.getElementById("btn-settings-footer");

  // Create Project Modal controls
  const createProjModal = document.getElementById("create-project-modal");
  const btnCloseProjModal = document.getElementById("btn-close-project-modal");
  const btnModalAddFolder = document.getElementById("btn-modal-add-folder");
  const btnModalSkipProject = document.getElementById("btn-modal-skip-project");
  const btnModalSaveProject = document.getElementById("btn-modal-save-project");
  const projNameInput = document.getElementById("project-name-input");
  const createProjFoldersList = document.getElementById("create-project-folders-list");

  // Click handler for settings footer
  if (btnSettingsFooter) {
    btnSettingsFooter.addEventListener("click", () => {
      document.getElementById("settings-overlay").classList.add("open");
      refreshSettingsUI();
      renderSettingsProjectsNav();
    });
  }

  // Toggle projects contextual menu
  if (btnProjectPlus && projectPlusDropdown) {
    btnProjectPlus.addEventListener("click", (e) => {
      e.stopPropagation();
      projectPlusDropdown.classList.toggle("open");
    });
    
    document.addEventListener("click", () => {
      projectPlusDropdown.classList.remove("open");
    });
  }

  // Open Create Project Modal
  if (menuNewProject && createProjModal) {
    menuNewProject.addEventListener("click", () => {
      createProjModal.classList.add("open");
      projNameInput.value = "";
      selectedFoldersForNewProject = [];
      renderNewProjectFolders();
    });
  }

  // Close Create Project Modal
  if (btnCloseProjModal && createProjModal) {
    btnCloseProjModal.addEventListener("click", () => {
      createProjModal.classList.remove("open");
    });
  }
  if (btnModalSkipProject && createProjModal) {
    btnModalSkipProject.addEventListener("click", () => {
      createProjModal.classList.remove("open");
    });
  }

  // Modal Add Folder
  if (btnModalAddFolder) {
    btnModalAddFolder.addEventListener("click", () => {
      if (!window.__TAURI__) return;
      const { invoke } = window.__TAURI__.core;
      invoke("select_folder")
        .then(folder => {
          if (folder) {
            if (!selectedFoldersForNewProject.includes(folder)) {
              selectedFoldersForNewProject.push(folder);
              renderNewProjectFolders();
            }
          }
        })
        .catch(err => {
          alert("選取資料夾失敗: " + err);
        });
    });
  }

  // Modal Save Project
  if (btnModalSaveProject) {
    btnModalSaveProject.addEventListener("click", () => {
      const name = projNameInput.value.trim();
      if (!name) {
        alert("請輸入專案名稱！");
        return;
      }
      if (!window.__TAURI__) return;
      const { invoke } = window.__TAURI__.core;
      invoke("create_project", { name, folders: selectedFoldersForNewProject })
        .then(projectId => {
          createProjModal.classList.remove("open");
          activeProjectId = projectId;
          // Set workspace path to first folder if exists
          if (selectedFoldersForNewProject.length > 0) {
            activeWorkspace = selectedFoldersForNewProject[0];
            document.getElementById("workspace-path-input").value = activeWorkspace;
            loadFileTree(activeWorkspace);
          }
          loadProjectsAndHistory();
        })
        .catch(err => {
          alert("建立專案失敗: " + err);
        });
    });
  }

  // Quick Start Menu Item
  if (menuQuickStart) {
    menuQuickStart.addEventListener("click", () => {
      if (!window.__TAURI__) return;
      const { invoke } = window.__TAURI__.core;
      invoke("select_folder")
        .then(folder => {
          if (folder) {
            activeWorkspace = folder;
            activeProjectId = null;
            document.getElementById("workspace-path-input").value = activeWorkspace;
            loadFileTree(activeWorkspace);
            // Open new chat
            startNewChat();
          }
        });
    });
  }

  // New Conversation Button
  if (btnNewChat) {
    btnNewChat.addEventListener("click", () => {
      startNewChat();
    });
  }

  // Sidebar Buttons
  if (sidebarBtnHistory) {
    sidebarBtnHistory.addEventListener("click", () => {
      document.querySelectorAll(".sidebar-menu-items .menu-item").forEach(b => b.classList.remove("active"));
      sidebarBtnHistory.classList.add("active");
    });
  }
  if (sidebarBtnScheduled) {
    sidebarBtnScheduled.addEventListener("click", () => {
      alert("排程任務監控已開啟，目前無待執行排程任務。");
    });
  }

  // Initial load
  loadProjectsAndHistory();
}

function renderNewProjectFolders() {
  const container = document.getElementById("create-project-folders-list");
  const saveBtn = document.getElementById("btn-modal-save-project");
  const skipBtn = document.getElementById("btn-modal-skip-project");
  
  if (!container) return;
  
  if (selectedFoldersForNewProject.length === 0) {
    container.innerHTML = `<div class="text-muted" style="font-size:12px; text-align:center; padding:10px 0;">No folder selected</div>`;
    if (saveBtn) saveBtn.style.display = "none";
    if (skipBtn) skipBtn.style.display = "block";
  } else {
    container.innerHTML = selectedFoldersForNewProject.map((f, idx) => `
      <div class="folder-entry">
        <span>📁 ${escapeHtml(f)}</span>
        <button data-index="${idx}">×</button>
      </div>
    `).join("");
    
    container.querySelectorAll("button").forEach(btn => {
      btn.addEventListener("click", (e) => {
        const idx = parseInt(e.target.getAttribute("data-index"), 10);
        selectedFoldersForNewProject.splice(idx, 1);
        renderNewProjectFolders();
      });
    });
    
    if (saveBtn) saveBtn.style.display = "block";
    if (skipBtn) skipBtn.style.display = "none";
  }
}

function startNewChat() {
  activeTaskId = null;
  const messagesContainer = document.getElementById("chat-messages-container");
  const traceBody = document.getElementById("trace-logs-body");
  
  if (messagesContainer) {
    messagesContainer.innerHTML = `
      <div class="message system">
        <div class="msg-avatar">🛡️</div>
        <div class="msg-content">
          <h3>系統就緒</h3>
          <p>Agnes AI 核心已載入。本機狀態機資料庫與 Exit-Code 自動對齊機制已激活。請在下方輸入提示詞，我將自動撰寫程式碼並在防禦沙盒中編譯測試。</p>
        </div>
      </div>
    `;
  }
  
  if (traceBody) {
    traceBody.innerHTML = `<div class="console-line attempt-info">// 待命，等待指令執行...</div>`;
  }

  // Highlight new conversation or clear selected highlights
  document.querySelectorAll(".sidebar-conversation-item").forEach(item => item.classList.remove("active"));
  
  // Set breadcrumbs name
  const projName = activeProjectId ? (projects.find(p => p.id === activeProjectId)?.name || "Agnes-AI") : "Agnes-AI";
  document.getElementById("breadcrumb-project-name").textContent = projName;
  document.getElementById("breadcrumb-task-name").textContent = "🌿 main";
}

function formatTime(dateStr) {
  if (!dateStr) return "now";
  // Convert standard sqlite timestamp to ISO format
  const t = dateStr.replace(" ", "T");
  const date = new Date(t);
  const now = new Date();
  const diffMs = now - date;
  if (isNaN(diffMs) || diffMs < 0) return "now";
  
  const diffSecs = Math.floor(diffMs / 1000);
  const diffMins = Math.floor(diffSecs / 60);
  const diffHours = Math.floor(diffMins / 60);
  const diffDays = Math.floor(diffHours / 24);
  
  if (diffMins < 1) return "now";
  if (diffMins < 60) return `${diffMins}m`;
  if (diffHours < 24) return `${diffHours}h`;
  return `${diffDays}d`;
}

function loadProjectsAndHistory() {
  if (!window.__TAURI__) return;
  const { invoke } = window.__TAURI__.core;
  
  Promise.all([
    invoke("get_projects"),
    invoke("get_tasks")
  ])
    .then(([projectsList, tasksList]) => {
      projects = projectsList;
      renderProjectsSidebar(projectsList, tasksList);
    })
    .catch(err => {
      console.error("載入專案與歷史失敗: ", err);
    });
}

function renderProjectsSidebar(projectsList, tasksList) {
  const container = document.getElementById("projects-list");
  if (!container) return;
  
  if (projectsList.length === 0 && tasksList.length === 0) {
    container.innerHTML = `<div class="text-muted" style="font-size:12px; text-align:center; padding:20px 0;">尚未建立專案</div>`;
    return;
  }

  // Group tasks by project_id
  const tasksByProject = {};
  const unclassifiedTasks = [];
  
  tasksList.forEach(task => {
    if (task.project_id) {
      if (!tasksByProject[task.project_id]) {
        tasksByProject[task.project_id] = [];
      }
      tasksByProject[task.project_id].push(task);
    } else {
      unclassifiedTasks.push(task);
    }
  });

  let html = "";
  
  // Render Projects
  projectsList.forEach(project => {
    const projTasks = tasksByProject[project.id] || [];
    // Keep it expanded if selected or has active task
    const isCollapsed = activeProjectId !== project.id && !projTasks.some(t => t.id === activeTaskId);
    
    html += `
      <div class="project-tree-item ${isCollapsed ? 'collapsed' : ''}" id="project-node-${project.id}" data-project-id="${project.id}">
        <div class="project-node-header">
          <span class="chevron">▼</span>
          <span class="folder-icon">📁</span>
          <span class="project-name" title="${escapeHtml(project.name)}">${escapeHtml(project.name)}</span>
        </div>
        <div class="project-node-conversations">
          ${projTasks.map(task => `
            <div class="sidebar-conversation-item ${activeTaskId === task.id ? 'active' : ''}" data-task-id="${task.id}" data-project-id="${project.id}">
              <span class="chat-icon">📝</span>
              <span class="chat-name" title="${escapeHtml(task.name)}">${escapeHtml(task.name)}</span>
              <span class="chat-time">${formatTime(task.created_at)}</span>
            </div>
          `).join("")}
          ${projTasks.length === 0 ? '<div class="text-muted" style="font-size:11px; padding: 4px 8px;">無對話</div>' : ''}
        </div>
      </div>
    `;
  });

  // Render Unclassified Tasks if any
  if (unclassifiedTasks.length > 0) {
    const isCollapsed = activeProjectId !== "unclassified";
    html += `
      <div class="project-tree-item ${isCollapsed ? 'collapsed' : ''}" id="project-node-unclassified" data-project-id="unclassified">
        <div class="project-node-header">
          <span class="chevron">▼</span>
          <span class="folder-icon">📁</span>
          <span class="project-name">Quick Starts</span>
        </div>
        <div class="project-node-conversations">
          ${unclassifiedTasks.map(task => `
            <div class="sidebar-conversation-item ${activeTaskId === task.id ? 'active' : ''}" data-task-id="${task.id}" data-project-id="unclassified">
              <span class="chat-icon">📝</span>
              <span class="chat-name" title="${escapeHtml(task.name)}">${escapeHtml(task.name)}</span>
              <span class="chat-time">${formatTime(task.created_at)}</span>
            </div>
          `).join("")}
        </div>
      </div>
    `;
  }

  container.innerHTML = html;

  // Bind Collapse/Expand toggles
  container.querySelectorAll(".project-node-header").forEach(header => {
    header.addEventListener("click", (e) => {
      e.stopPropagation();
      const node = header.closest(".project-tree-item");
      const pid = node.getAttribute("data-project-id");
      node.classList.toggle("collapsed");
      
      // Set as active project on clicking header
      if (pid && pid !== "unclassified") {
        activeProjectId = pid;
        const project = projects.find(p => p.id === pid);
        if (project && project.folders) {
          const folders = JSON.parse(project.folders);
          if (folders.length > 0) {
            activeWorkspace = folders[0];
            document.getElementById("workspace-path-input").value = activeWorkspace;
            loadFileTree(activeWorkspace);
            
            // Update breadcrumb
            document.getElementById("breadcrumb-project-name").textContent = project.name;
          }
        }
      } else {
        activeProjectId = null;
      }
    });
  });

  // Bind click event on conversation item
  container.querySelectorAll(".sidebar-conversation-item").forEach(item => {
    item.addEventListener("click", (e) => {
      e.stopPropagation();
      const taskId = item.getAttribute("data-task-id");
      const pid = item.getAttribute("data-project-id");
      
      activeTaskId = taskId;
      activeProjectId = pid === "unclassified" ? null : pid;
      
      // Highlight active in UI
      container.querySelectorAll(".sidebar-conversation-item").forEach(el => el.classList.remove("active"));
      item.classList.add("active");
      
      // Select conversation project folder if exists
      if (activeProjectId) {
        const project = projects.find(p => p.id === activeProjectId);
        if (project && project.folders) {
          const folders = JSON.parse(project.folders);
          if (folders.length > 0) {
            activeWorkspace = folders[0];
            document.getElementById("workspace-path-input").value = activeWorkspace;
            loadFileTree(activeWorkspace);
            document.getElementById("breadcrumb-project-name").textContent = project.name;
          }
        }
      }
      
      loadConversationData(taskId);
    });
  });
}

function loadConversationData(taskId) {
  if (!window.__TAURI__) return;
  const { invoke } = window.__TAURI__.core;
  const messagesContainer = document.getElementById("chat-messages-container");
  const traceBody = document.getElementById("trace-logs-body");
  
  messagesContainer.innerHTML = `<div class="text-muted text-center py-20">載入歷史紀錄中...</div>`;
  traceBody.innerHTML = `<div class="console-line font-mono">// 載入日誌軌跡中...</div>`;

  // Fetch execution logs, audits and tasks details
  Promise.all([
    invoke("get_tasks"), // to find the specific task name/payload
    invoke("get_task_logs", { taskId }),
    invoke("get_audit_logs", { taskId })
  ])
    .then(([tasksList, taskLogs, auditLogs]) => {
      const task = tasksList.find(t => t.id === taskId);
      if (!task) {
        messagesContainer.innerHTML = `<div class="text-muted text-center py-20">找不到對話</div>`;
        return;
      }

      // Update breadcrumb task name
      document.getElementById("breadcrumb-task-name").textContent = `🌿 ${task.name}`;

      // Reset messagesContainer
      messagesContainer.innerHTML = `
        <div class="message system">
          <div class="msg-avatar">🛡️</div>
          <div class="msg-content">
            <h3>系統就緒</h3>
            <p>Agnes AI 核心已載入。本機狀態機資料庫與 Exit-Code 自動對齊機制已激活。</p>
          </div>
        </div>
      `;

      // Render user prompt
      messagesContainer.innerHTML += `
        <div class="message user">
          <div class="msg-avatar">Y</div>
          <div class="msg-content">
            <p>${escapeHtml(task.payload)}</p>
          </div>
        </div>
      `;

      // Reconstruct assistant response if logs exist
      if (taskLogs.length > 0) {
        const assistantMsgId = "agent-reply-history-" + taskId;
        let assistantContent = `
          <div class="message assistant" id="${assistantMsgId}">
            <div class="msg-avatar">🤖</div>
            <div class="msg-content">
              <h3>Agnes AI</h3>
        `;

        // Render each tool execution details inside
        taskLogs.forEach(log => {
          const toolClass = log.exit_code && log.exit_code !== 0 ? 'style="border-color:var(--failed);"' : '';
          const titleClass = log.exit_code && log.exit_code !== 0 ? 'style="color:var(--failed); font-weight:bold;"' : '';
          
          assistantContent += `
            <div class="tool-trace-block" ${toolClass}>
              <div class="trace-title" ${titleClass}>🔧 ${escapeHtml(log.command)}</div>
              <div class="trace-result">${escapeHtml(log.stdout || log.stderr || "無輸出")}</div>
            </div>
          `;
        });

        assistantContent += `
            </div>
          </div>
        `;
        
        messagesContainer.innerHTML += assistantContent;
      } else {
        messagesContainer.innerHTML += `
          <div class="message assistant">
            <div class="msg-avatar">🤖</div>
            <div class="msg-content">
              <h3>Agnes AI</h3>
              <p>無已儲存的執行紀錄。</p>
            </div>
          </div>
        `;
      }

      // Render audit logs to trace body
      if (auditLogs.length > 0) {
        let traceHtml = `<div class="console-line attempt-info">// 載入歷史審查軌跡...</div>`;
        auditLogs.forEach(audit => {
          const isPassed = audit.verdict === "PASSED";
          if (isPassed) {
            traceHtml += `<div class="console-line pass">// ${audit.agent_name}: 通過 (${audit.reason})</div>`;
          } else {
            traceHtml += `<div class="console-line reject" style="color:var(--failed); font-weight:bold;">// ${audit.agent_name}: 否決 (${audit.reason})</div>`;
          }
        });
        traceBody.innerHTML = traceHtml;
      } else {
        traceBody.innerHTML = `<div class="console-line font-mono">// 此對話無子代理人審查紀錄。</div>`;
      }
      
      messagesContainer.scrollTop = messagesContainer.scrollHeight;
    })
    .catch(err => {
      messagesContainer.innerHTML = `<div class="text-muted text-center py-20" style="color:var(--failed);">載入錯誤: ${escapeHtml(err)}</div>`;
    });
}

// Render subagents inside sidebar
function initSubagents() {
  const list = document.getElementById("subagents-sidebar-list");
  if (!list) return;

  list.innerHTML = subagents.map(agent => `
    <div class="subagent-item" id="sidebar-agent-${agent.id}">
      <span class="subagent-name">${agent.role}</span>
      <span class="subagent-status sleeping" id="sidebar-agent-status-${agent.id}">sleeping</span>
    </div>
  `).join("");
}

function setAgentState(agentName, state) {
  const agent = subagents.find(a => a.name === agentName);
  if (!agent) return;

  const tag = document.getElementById(`sidebar-agent-status-${agent.id}`);
  if (tag) {
    tag.textContent = state;
    if (state === "active" || state === "passed") {
      tag.className = "subagent-status active";
      tag.style.backgroundColor = "var(--success-bg)";
      tag.style.color = "var(--success)";
    } else if (state === "auditing") {
      tag.className = "subagent-status active";
      tag.style.backgroundColor = "var(--warning-bg)";
      tag.style.color = "var(--warning)";
    } else if (state === "rejected" || state === "failed") {
      tag.className = "subagent-status active";
      tag.style.backgroundColor = "var(--failed-bg)";
      tag.style.color = "var(--failed)";
    } else {
      tag.className = "subagent-status sleeping";
      tag.style.backgroundColor = "rgba(255, 255, 255, 0.03)";
      tag.style.color = "var(--text-muted)";
    }
  }
}

function updatePermissionBadge(fullAccess) {
  const badgeEl = document.getElementById("permission-dropdown");
  if (!badgeEl) return;
  if (fullAccess) {
    badgeEl.innerHTML = `<span class="badge badge-warning">🔓 完整存取權已開啟</span>`;
  } else {
    badgeEl.innerHTML = `<span class="badge badge-muted" style="background:rgba(255,255,255,0.05); color:var(--text-secondary);">🔒 限制工作區存取</span>`;
  }
}

function refreshSettingsUI() {
  if (!window.__TAURI__) return;
  const { invoke } = window.__TAURI__.core;
  const keyInput = document.getElementById("settings-key");
  const defaultPermissionsCheckbox = document.getElementById("settings-default-permissions");
  const autoReviewCheckbox = document.getElementById("settings-auto-review");
  const fullAccessCheckbox = document.getElementById("settings-full-access");
  const shellSelect = document.getElementById("settings-shell");
  const langSelect = document.getElementById("settings-lang");
  const timeoutInput = document.getElementById("settings-timeout-num");
  const retriesInput = document.getElementById("settings-retries-num");
  const workModeCards = document.querySelectorAll(".work-mode-card");

  invoke("get_config")
    .then(config => {
      keyInput.value = config.api.key;
      timeoutInput.value = config.sandbox.timeout_seconds;
      retriesInput.value = config.sandbox.max_retries;
      
      defaultPermissionsCheckbox.checked = config.security.default_permissions;
      autoReviewCheckbox.checked = config.security.auto_review;
      fullAccessCheckbox.checked = config.security.full_access;
      
      updatePermissionBadge(config.security.full_access);

      shellSelect.value = config.general.shell;
      langSelect.value = config.general.language;
      
      workModeCards.forEach(c => {
        if (c.getAttribute("data-work-mode") === config.general.work_mode) {
          c.classList.add("active");
        } else {
          c.classList.remove("active");
        }
      });

      loadMcpTools();
      loadMcpServersList(config.mcp_servers);
    })
    .catch(err => {
      console.error("載入組態失敗", err);
    });
}

function initSettings() {
  const keyInput = document.getElementById("settings-key");
  const defaultPermissionsCheckbox = document.getElementById("settings-default-permissions");
  const autoReviewCheckbox = document.getElementById("settings-auto-review");
  const fullAccessCheckbox = document.getElementById("settings-full-access");
  const shellSelect = document.getElementById("settings-shell");
  const langSelect = document.getElementById("settings-lang");
  const timeoutInput = document.getElementById("settings-timeout-num");
  const retriesInput = document.getElementById("settings-retries-num");
  const saveGeneralBtn = document.getElementById("btn-save-general-settings");

  document.getElementById("btn-close-settings").addEventListener("click", () => {
    document.getElementById("settings-overlay").classList.remove("open");
  });

  // Modal sidebar navigation
  const modalNavButtons = document.querySelectorAll(".settings-nav-btn");
  const modalPanes = document.querySelectorAll(".settings-pane");
  modalNavButtons.forEach(btn => {
    btn.addEventListener("click", () => {
      modalNavButtons.forEach(b => b.classList.remove("active"));
      modalPanes.forEach(p => p.classList.remove("active"));
      
      btn.classList.add("active");
      const targetPane = btn.getAttribute("data-settings-pane");
      document.getElementById(targetPane).classList.add("active");
    });
  });

  // Work mode card toggle
  const workModeCards = document.querySelectorAll(".work-mode-card");
  workModeCards.forEach(card => {
    card.addEventListener("click", () => {
      workModeCards.forEach(c => c.classList.remove("active"));
      card.classList.add("active");
    });
  });

  // Code review button group
  const reviewInlineBtn = document.getElementById("btn-review-inline");
  const reviewIndependentBtn = document.getElementById("btn-review-independent");
  reviewInlineBtn.addEventListener("click", () => {
    reviewInlineBtn.classList.add("active");
    reviewIndependentBtn.classList.remove("active");
  });
  reviewIndependentBtn.addEventListener("click", () => {
    reviewIndependentBtn.classList.add("active");
    reviewInlineBtn.classList.remove("active");
  });

  // Save Settings Click
  saveGeneralBtn.addEventListener("click", () => {
    if (!window.__TAURI__) return;
    const { invoke } = window.__TAURI__.core;
    
    const key = keyInput.value;
    const timeout = parseInt(timeoutInput.value, 10);
    const retries = parseInt(retriesInput.value, 10);
    const requireApproval = !autoReviewCheckbox.checked;
    const defaultPermissions = defaultPermissionsCheckbox.checked;
    const autoReview = autoReviewCheckbox.checked;
    const fullAccess = fullAccessCheckbox.checked;
    
    let workMode = "programming";
    workModeCards.forEach(c => {
      if (c.classList.contains("active")) {
        workMode = c.getAttribute("data-work-mode");
      }
    });
    
    const shell = shellSelect.value;
    const language = langSelect.value;

    invoke("save_config", {
      key,
      timeoutSeconds: timeout,
      maxRetries: retries,
      requireApproval,
      defaultPermissions,
      autoReview,
      fullAccess,
      workMode,
      shell,
      language
    })
      .then(() => {
        alert("一般設定儲存成功！已寫入 config.local.toml");
        updatePermissionBadge(fullAccess);
        loadMcpTools();
      })
      .catch(err => {
        alert("儲存設定失敗: " + err);
      });
  });

  // MCP dynamic add form bindings
  const btnShowAddForm = document.getElementById("btn-show-add-mcp-form");
  const addFormContainer = document.getElementById("add-mcp-form-container");
  const btnCancelAdd = document.getElementById("btn-cancel-add-mcp");
  const btnSubmitAdd = document.getElementById("btn-submit-add-mcp");
  
  btnShowAddForm.addEventListener("click", () => {
    addFormContainer.style.display = "block";
    btnShowAddForm.style.display = "none";
  });
  
  btnCancelAdd.addEventListener("click", () => {
    addFormContainer.style.display = "none";
    btnShowAddForm.style.display = "block";
  });
  
  btnSubmitAdd.addEventListener("click", () => {
    const name = document.getElementById("add-mcp-name").value.trim();
    const command = document.getElementById("add-mcp-command").value.trim();
    const argsStr = document.getElementById("add-mcp-args").value.trim();
    
    if (!name || !command) {
      alert("請填寫伺服器名稱與指令！");
      return;
    }
    
    const args = argsStr ? argsStr.split(/\s+/) : [];
    
    if (!window.__TAURI__) return;
    const { invoke } = window.__TAURI__.core;
    invoke("add_mcp_server", { name, command, args })
      .then(() => {
        alert("新增 MCP 伺服器成功！");
        document.getElementById("add-mcp-name").value = "";
        document.getElementById("add-mcp-command").value = "";
        document.getElementById("add-mcp-args").value = "";
        addFormContainer.style.display = "none";
        btnShowAddForm.style.display = "block";
        
        invoke("get_config")
          .then(config => {
            loadMcpTools();
            loadMcpServersList(config.mcp_servers);
          });
      })
      .catch(err => {
        alert(`新增伺服器失敗: ${err}`);
      });
  });

  // Project settings panel override event listeners
  const btnEditProjName = document.getElementById("btn-edit-proj-name");
  const btnDeleteProj = document.getElementById("btn-delete-proj");
  const btnProjAddFolder = document.getElementById("btn-proj-add-folder");
  const btnSaveProjSettingsDb = document.getElementById("btn-save-project-settings-db");
  const btnProjFileRules = document.getElementById("btn-proj-file-rules");

  if (btnEditProjName) {
    btnEditProjName.addEventListener("click", () => {
      if (!activeSettingProjectId) return;
      const proj = projects.find(p => p.id === activeSettingProjectId);
      if (!proj) return;
      const newName = prompt("請輸入新的專案名稱：", proj.name);
      if (newName && newName.trim()) {
        if (!window.__TAURI__) return;
        const { invoke } = window.__TAURI__.core;
        invoke("update_project_name", { id: activeSettingProjectId, name: newName.trim() })
          .then(() => {
            proj.name = newName.trim();
            document.getElementById("proj-settings-title").textContent = newName.trim();
            loadProjectsAndHistory();
          })
          .catch(err => alert("更新名稱失敗: " + err));
      }
    });
  }

  if (btnDeleteProj) {
    btnDeleteProj.addEventListener("click", () => {
      if (!activeSettingProjectId) return;
      if (confirm("您確定要刪除此專案嗎？這將會解除所有該專案下對話的關聯。")) {
        if (!window.__TAURI__) return;
        const { invoke } = window.__TAURI__.core;
        invoke("delete_project", { id: activeSettingProjectId })
          .then(() => {
            document.getElementById("settings-overlay").classList.remove("open");
            activeProjectId = null;
            activeTaskId = null;
            loadProjectsAndHistory();
            startNewChat();
          })
          .catch(err => alert("刪除專案失敗: " + err));
      }
    });
  }

  if (btnProjAddFolder) {
    btnProjAddFolder.addEventListener("click", () => {
      if (!window.__TAURI__) return;
      const { invoke } = window.__TAURI__.core;
      invoke("select_folder")
        .then(folder => {
          if (folder) {
            if (!selectedFoldersForCurrentProject.includes(folder)) {
              selectedFoldersForCurrentProject.push(folder);
              renderCurrentProjectFolders();
            }
          }
        })
        .catch(err => alert("選取資料夾失敗: " + err));
    });
  }

  if (btnSaveProjSettingsDb) {
    btnSaveProjSettingsDb.addEventListener("click", () => {
      if (!activeSettingProjectId) return;
      if (!window.__TAURI__) return;
      const { invoke } = window.__TAURI__.core;

      const preset = document.getElementById("proj-security-preset").value;
      const policy = document.getElementById("proj-review-policy").value;

      let requireApproval = true;
      let defaultPermissions = true;
      let autoReview = false;
      let fullAccess = false;

      if (preset === "turbo") {
        requireApproval = false;
        defaultPermissions = true;
        autoReview = true;
        fullAccess = true;
      } else if (preset === "strict") {
        requireApproval = true;
        defaultPermissions = false;
        autoReview = false;
        fullAccess = false;
      }

      if (policy === "always") {
        autoReview = true;
      }

      const wm = document.getElementById("proj-work-mode").value;
      const sh = document.getElementById("proj-shell").value;
      const lg = document.getElementById("proj-lang").value;

      const workMode = wm === "none" ? null : wm;
      const shell = sh === "none" ? null : sh;
      const language = lg === "none" ? null : lg;

      Promise.all([
        invoke("save_project_settings", {
          id: activeSettingProjectId,
          workMode,
          shell,
          language,
          requireApproval,
          defaultPermissions,
          autoReview,
          fullAccess
        }),
        invoke("save_project_folders", {
          id: activeSettingProjectId,
          folders: selectedFoldersForCurrentProject
        })
      ])
        .then(() => {
          alert("專案設定儲存成功！");
          if (activeProjectId === activeSettingProjectId) {
            if (selectedFoldersForCurrentProject.length > 0) {
              activeWorkspace = selectedFoldersForCurrentProject[0];
              document.getElementById("workspace-path-input").value = activeWorkspace;
              loadFileTree(activeWorkspace);
            }
          }
          loadProjectsAndHistory();
        })
        .catch(err => alert("儲存專案設定失敗: " + err));
    });
  }

  if (btnProjFileRules) {
    btnProjFileRules.addEventListener("click", () => {
      alert("本機檔案存取規則配置（File Access Rules）正常載入中。");
    });
  }

  // Load initially
  refreshSettingsUI();
}

function renderSettingsProjectsNav() {
  const container = document.getElementById("settings-projects-list-nav");
  if (!container) return;
  
  if (projects.length === 0) {
    container.innerHTML = `<div class="text-muted" style="font-size:11px; padding: 6px 14px;">No projects</div>`;
    return;
  }
  
  container.innerHTML = projects.map(p => `
    <button class="settings-nav-btn project-nav-btn-item" data-settings-pane="project-settings-pane" data-project-id="${p.id}">
      <span class="project-nav-item">
        <span class="icon">📁</span>
        <span>${escapeHtml(p.name)}</span>
      </span>
    </button>
  `).join("");
  
  container.querySelectorAll(".project-nav-btn-item").forEach(btn => {
    btn.addEventListener("click", () => {
      document.querySelectorAll(".settings-nav-btn").forEach(b => b.classList.remove("active"));
      btn.classList.add("active");
      
      // Hide all standard panes
      document.querySelectorAll(".settings-pane").forEach(p => p.classList.remove("active"));
      
      // Show project settings pane
      const pane = document.getElementById("project-settings-pane");
      if (pane) pane.classList.add("active");
      
      const projectId = btn.getAttribute("data-project-id");
      showProjectSettings(projectId);
    });
  });
}

function showProjectSettings(projectId) {
  const project = projects.find(p => p.id === projectId);
  if (!project) return;
  
  activeSettingProjectId = projectId;
  document.getElementById("proj-settings-title").textContent = project.name;
  
  try {
    selectedFoldersForCurrentProject = JSON.parse(project.folders || "[]");
  } catch (e) {
    selectedFoldersForCurrentProject = [];
  }
  
  renderCurrentProjectFolders();
  
  // Set preset
  const presetSelect = document.getElementById("proj-security-preset");
  if (project.full_access && project.auto_review && !project.require_approval) {
    presetSelect.value = "turbo";
  } else if (!project.full_access && project.require_approval && !project.default_permissions) {
    presetSelect.value = "strict";
  } else {
    presetSelect.value = "standard";
  }
  
  // Set policy
  const policySelect = document.getElementById("proj-review-policy");
  policySelect.value = project.auto_review ? "always" : "require";
  
  // Set overrides
  document.getElementById("proj-work-mode").value = project.work_mode || "none";
  document.getElementById("proj-shell").value = project.shell || "none";
  document.getElementById("proj-lang").value = project.language || "none";
}

function renderCurrentProjectFolders() {
  const container = document.getElementById("proj-settings-folders");
  if (!container) return;
  
  if (selectedFoldersForCurrentProject.length === 0) {
    container.innerHTML = `<div class="text-muted" style="font-size:12px; text-align:center; padding:10px 0;">No folders added</div>`;
  } else {
    container.innerHTML = selectedFoldersForCurrentProject.map((f, idx) => `
      <div class="proj-folder-item">
        <span>📁 ${escapeHtml(f)}</span>
        <button data-index="${idx}">×</button>
      </div>
    `).join("");
    
    container.querySelectorAll("button").forEach(btn => {
      btn.addEventListener("click", (e) => {
        const idx = parseInt(e.target.getAttribute("data-index"), 10);
        selectedFoldersForCurrentProject.splice(idx, 1);
        renderCurrentProjectFolders();
      });
    });
  }
}

function loadMcpTools() {
  if (!window.__TAURI__) return;
  const { invoke } = window.__TAURI__.core;
  const mcpList = document.getElementById("mcp-servers-list");

  invoke("get_mcp_tools")
    .then(servers => {
      const keys = Object.keys(servers);
      if (keys.length === 0) {
        mcpList.innerHTML = `<div class="text-muted" style="font-size:11px;">無啟用的 MCP 伺服器</div>`;
        return;
      }
      
      mcpList.innerHTML = keys.map(srvName => {
        const tools = servers[srvName];
        return `
          <div class="mcp-server-item">
            <div class="mcp-server-name-header">
              <span>🖥️ ${srvName}</span>
              <span class="mcp-tools-badge">${tools.length} Tools</span>
            </div>
            <div class="mcp-server-tools-list">
              ${tools.map(t => `
                <div class="mcp-tool-detail" title="${escapeHtml(t.description)}">
                  ⚡ ${t.name}
                </div>
              `).join("")}
            </div>
          </div>
        `;
      }).join("");
    })
    .catch(err => {
      console.error("載入 MCP 工具失敗", err);
    });
}

function loadMcpServersList(mcpServers) {
  const cardsList = document.getElementById("mcp-server-cards-list");
  if (!cardsList) return;
  
  if (!mcpServers || mcpServers.length === 0) {
    cardsList.innerHTML = `<div class="text-muted" style="font-size:13px;">尚未配置自訂的 MCP 伺服器</div>`;
    return;
  }
  
  cardsList.innerHTML = mcpServers.map(server => {
    return `
      <div class="mcp-card">
        <div class="mcp-card-details">
          <h4>🖥️ ${escapeHtml(server.name)}</h4>
          <p>${escapeHtml(server.command)} ${escapeHtml(server.args.join(" "))}</p>
        </div>
        <div class="mcp-card-actions">
          <button class="btn-gear" title="設定">⚙️</button>
          <label class="switch">
            <input type="checkbox" class="mcp-toggle" data-server-name="${escapeHtml(server.name)}" ${server.enabled ? "checked" : ""}>
            <span class="slider"></span>
          </label>
        </div>
      </div>
    `;
  }).join("");
  
  const toggles = cardsList.querySelectorAll(".mcp-toggle");
  toggles.forEach(toggle => {
    toggle.addEventListener("change", (e) => {
      const serverName = e.target.getAttribute("data-server-name");
      const enabled = e.target.checked;
      
      if (!window.__TAURI__) return;
      const { invoke } = window.__TAURI__.core;
      invoke("toggle_mcp_server", { name: serverName, enabled })
        .then(() => {
          loadMcpTools();
        })
        .catch(err => {
          alert(`切換伺服器失敗: ${err}`);
          e.target.checked = !enabled;
        });
    });
  });
}

// Workspace file tree loader
function initWorkspace() {
  const pathInput = document.getElementById("workspace-path-input");
  const loadBtn = document.getElementById("btn-load-workspace");

  loadBtn.addEventListener("click", () => {
    activeWorkspace = pathInput.value;
    loadFileTree(activeWorkspace);
  });

  loadFileTree(activeWorkspace);
}

function loadFileTree(workspacePath) {
  if (!window.__TAURI__) return;
  const { invoke } = window.__TAURI__.core;
  const treeContainer = document.getElementById("file-tree");

  treeContainer.innerHTML = `<div class="text-muted text-center py-20">載入中...</div>`;

  invoke("read_directory", { dirPath: workspacePath })
    .then(files => {
      if (files.length === 0) {
        treeContainer.innerHTML = `<div class="text-muted text-center py-20">目錄為空或無存取權</div>`;
        return;
      }
      treeContainer.innerHTML = files.map(file => `
        <div class="file-item" title="${file}">
          📄 ${file}
        </div>
      `).join("");
    })
    .catch(err => {
      treeContainer.innerHTML = `<div class="text-muted text-center py-20" style="color:var(--failed);">載入失敗: ${escapeHtml(err)}</div>`;
    });
}

// Collapsible Trace panel
function initCollapsibleTrace() {
  const header = document.getElementById("execution-trace");
  const toggleBtn = document.getElementById("btn-toggle-trace");

  header.querySelector(".trace-header").addEventListener("click", () => {
    header.classList.toggle("collapsed");
    toggleBtn.textContent = header.classList.contains("collapsed") ? "展開日誌" : "收合";
  });
}

// Open IDE explorer command
function initIDEButton() {
  const btn = document.getElementById("btn-open-ide");
  btn.addEventListener("click", () => {
    if (!window.__TAURI__) return;
    const { invoke } = window.__TAURI__.core;
    invoke("execute_task", {
      name: "開啟專案資料夾",
      program: "explorer",
      args: [activeWorkspace]
    }).catch(err => {
      console.error("無法開啟 IDE 資料夾: ", err);
    });
  });
}

// Chat agent loop integration
function initChatAgent() {
  const promptInput = document.getElementById("prompt-input");
  const sendBtn = document.getElementById("btn-send-message");
  const messagesContainer = document.getElementById("chat-messages-container");
  const traceBody = document.getElementById("trace-logs-body");
  const approvalPanel = document.getElementById("approval-panel");
  const approvalToolsList = document.getElementById("approval-tools-list");
  const approveBtn = document.getElementById("btn-approve-step");
  const rejectBtn = document.getElementById("btn-reject-step");
  const rejectionFeedbackContainer = document.getElementById("rejection-feedback-container");
  const rejectionFeedbackInput = document.getElementById("rejection-feedback-input");
  const submitRejectionBtn = document.getElementById("btn-submit-rejection");

  promptInput.addEventListener("input", () => {
    promptInput.style.height = "auto";
    promptInput.style.height = promptInput.scrollHeight + "px";
  });

  promptInput.addEventListener("keydown", (e) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      sendMessage();
    }
  });

  sendBtn.addEventListener("click", sendMessage);

  approveBtn.addEventListener("click", () => {
    approvalPanel.style.display = "none";
    if (!window.__TAURI__) return;
    const { invoke } = window.__TAURI__.core;

    traceBody.innerHTML += `<div class="console-line attempt-info">// 使用者已授權執行，開始執行工具...</div>`;
    invoke("approve_pending_step")
      .then(step => {
        processStepResult(step);
      })
      .catch(err => {
        handleError(err);
      });
  });

  rejectBtn.addEventListener("click", () => {
    rejectionFeedbackContainer.style.display = "block";
  });

  submitRejectionBtn.addEventListener("click", () => {
    const feedback = rejectionFeedbackInput.value.trim() || "User rejected execution.";
    approvalPanel.style.display = "none";
    rejectionFeedbackContainer.style.display = "none";
    rejectionFeedbackInput.value = "";
    
    if (!window.__TAURI__) return;
    const { invoke } = window.__TAURI__.core;

    traceBody.innerHTML += `<div class="console-line reject">[USER REJECTED] 反饋已送出，重新載入自愈機制...</div>`;
    invoke("reject_pending_step", { feedback })
      .then(step => {
        processStepResult(step);
      })
      .catch(err => {
        handleError(err);
      });
  });

  function sendMessage() {
    const prompt = promptInput.value.trim();
    if (!prompt) return;

    messagesContainer.innerHTML += `
      <div class="message user">
        <div class="msg-avatar">Y</div>
        <div class="msg-content">
          <p>${escapeHtml(prompt)}</p>
        </div>
      </div>
    `;

    promptInput.value = "";
    promptInput.style.height = "auto";
    messagesContainer.scrollTop = messagesContainer.scrollHeight;

    const agentMsgId = "agent-reply-" + Date.now();
    messagesContainer.innerHTML += `
      <div class="message assistant" id="${agentMsgId}">
        <div class="msg-avatar">🤖</div>
        <div class="msg-content">
          <h3>Agnes AI</h3>
          <p class="typing-placeholder">自主智慧體正在思考任務，準備調用工具...</p>
        </div>
      </div>
    `;
    messagesContainer.scrollTop = messagesContainer.scrollHeight;
    currentAgentReplyEl = document.getElementById(agentMsgId);

    const tracePanel = document.getElementById("execution-trace");
    tracePanel.classList.remove("collapsed");
    traceBody.innerHTML = `<div class="console-line attempt-info">// 啟動自主代碼寫入任務...</div>`;

    if (!window.__TAURI__) {
      setTimeout(() => {
        currentAgentReplyEl.querySelector(".msg-content").innerHTML = `
          <h3>Agnes AI (模擬環境)</h3>
          <p>已偵測到 Web 預覽環境。已自動建立專案模擬檔案。</p>
        `;
      }, 1500);
      return;
    }

    const { invoke } = window.__TAURI__.core;
    invoke("send_chat_agent", { prompt, workspacePath: activeWorkspace, projectId: activeProjectId || null })
      .then(step => {
        loadProjectsAndHistory(true);
        processStepResult(step);
      })
      .catch(err => {
        handleError(err);
      });
  }

  async function processStepResult(step) {
    const msgContent = currentAgentReplyEl.querySelector(".msg-content");
    
    // Check if it's the first response block of the step or appending
    if (msgContent.querySelector(".typing-placeholder")) {
      msgContent.innerHTML = `<h3>Agnes AI</h3>`;
    }

    // Render LLM explanation text
    msgContent.innerHTML += `<p>${markdownToHtml(step.response_text)}</p>`;
    messagesContainer.scrollTop = messagesContainer.scrollHeight;

    // Run the 17-subagent structured audits animation sequentially
    traceBody.innerHTML += `<div class="console-line attempt-info">// 啟動 17 子代理人協同審查機制 (Audit Review)...</div>`;
    
    for (let i = 0; i < step.audits.length; i++) {
      const audit = step.audits[i];
      setAgentState(audit.agent_name, "auditing");
      
      // Log to console trace
      traceBody.innerHTML += `<div class="console-line font-mono" style="color:var(--warning);">[AUDIT] 正在調度 ${audit.agent_name}...</div>`;
      traceBody.scrollTop = traceBody.scrollHeight;
      
      // Delay for premium visualization scans
      await new Promise(resolve => setTimeout(resolve, 80));

      const isPassed = audit.verdict === "PASSED";
      setAgentState(audit.agent_name, isPassed ? "passed" : "rejected");
      
      if (isPassed) {
        traceBody.innerHTML += `<div class="console-line pass">// ${audit.agent_name}: 通過 (${audit.reason})</div>`;
      } else {
        traceBody.innerHTML += `<div class="console-line reject" style="color:var(--failed); font-weight:bold;">// ${audit.agent_name}: 否決 (${audit.reason})</div>`;
      }
      traceBody.scrollTop = traceBody.scrollHeight;
    }

    // Delay to absorb results
    await new Promise(resolve => setTimeout(resolve, 300));

    const anyRejected = step.audits.some(a => a.verdict === "REJECTED");

    if (anyRejected) {
      // Show audit rejection block in chat
      msgContent.innerHTML += `
        <div class="tool-trace-block" style="border-color:var(--failed);">
          <div class="trace-title" style="color:var(--failed); font-weight:bold;">❌ 17人專家團隊安全審查未通過</div>
          <div class="trace-result" style="color:var(--failed); font-weight:normal;">
            已經攔截此次執行。系統正在啟動自愈並重新設計防禦性代碼...
          </div>
        </div>
      `;
      messagesContainer.scrollTop = messagesContainer.scrollHeight;

      // Automatically advance to the next self-healing step after delay
      setTimeout(() => {
        if (!window.__TAURI__) return;
        const { invoke } = window.__TAURI__.core;
        traceBody.innerHTML += `<div class="console-line attempt-info">// 審查攔截成功。自動觸發自愈覆寫...</div>`;
        invoke("approve_pending_step")
          .then(nextStep => {
            processStepResult(nextStep);
          })
          .catch(err => {
            handleError(err);
          });
      }, 1500);
      return;
    }

    // Show tool call details in chat message
    step.executed_tools.forEach((tool, tIdx) => {
      const toolResult = step.execution_results[tIdx] || "等待使用者授權執行...";
      msgContent.innerHTML += `
        <div class="tool-trace-block">
          <div class="trace-title">🔧 工具調用: ${tool.name} ${tool.path ? `path="${tool.path}"` : ""}</div>
          <div class="trace-result">${escapeHtml(toolResult)}</div>
        </div>
      `;
      
      traceBody.innerHTML += `
        <div class="console-line attempt-info">Tool Call: ${tool.name}</div>
        <div class="console-line stdout-log">${escapeHtml(toolResult)}</div>
      `;
    });
    
    traceBody.scrollTop = traceBody.scrollHeight;
    messagesContainer.scrollTop = messagesContainer.scrollHeight;

    // Refresh workspace file tree list
    loadFileTree(activeWorkspace);

    // Turn off subagent active states after a short duration
    setTimeout(() => {
      subagents.forEach(a => setAgentState(a.name, "sleeping"));
    }, 2000);

    if (step.requires_approval) {
      // Render approval panel with tool calls description
      approvalToolsList.innerHTML = step.executed_tools.map(tool => `
        <div class="approval-tool-item">
          <strong>${tool.name}</strong> ${tool.path ? `(${tool.path})` : ""}
          <pre style="margin-top:4px; font-size:11px; max-height:80px; overflow-y:auto; color:var(--text-secondary); background:rgba(0,0,0,0.2); padding:4px; border-radius:4px;">${escapeHtml(tool.content)}</pre>
        </div>
      `).join("");
      
      approvalPanel.style.display = "block";
    } else {
      // If no approval is required, and there are executed tools, automatically run the next tick
      if (step.executed_tools.length > 0) {
        setTimeout(() => {
          if (!window.__TAURI__) return;
          const { invoke } = window.__TAURI__.core;
          traceBody.innerHTML += `<div class="console-line attempt-info">// 自動執行下一個工作 Tick...</div>`;
          invoke("approve_pending_step")
            .then(nextStep => {
              processStepResult(nextStep);
            })
            .catch(err => {
              handleError(err);
            });
        }, 1500);
      } else {
        // Complete
        traceBody.innerHTML += `<div class="console-line pass">[FINISHED] 自主任務鏈已全部順利完成！</div>`;
        loadProjectsAndHistory();
      }
    }
  }

  function handleError(err) {
    const msgContent = currentAgentReplyEl.querySelector(".msg-content");
    msgContent.innerHTML += `
      <p style="color:var(--failed);">呼叫出錯: ${escapeHtml(err)}</p>
    `;
    traceBody.innerHTML += `<div class="console-line reject">[ERROR] ${escapeHtml(err)}</div>`;
    messagesContainer.scrollTop = messagesContainer.scrollHeight;
  }
}

// Simple parser to format markdown back to HTML paragraphs/code blocks
function markdownToHtml(text) {
  if (!text) return "";
  let html = text;
  
  // Code blocks: ```rust code ``` -> <pre>
  html = html.replace(/```([\s\S]*?)```/g, '<pre style="background:#050608; padding:12px; border-radius:6px; font-family:var(--font-mono); font-size:13px; margin:10px 0; overflow-x:auto; border:1px solid var(--border-color); color:#a5d6ff;"><code>$1</code></pre>');
  
  // Inline code: `code` -> <code>
  html = html.replace(/`([^`]+)`/g, '<code style="background:rgba(255,255,255,0.05); padding:2px 6px; border-radius:4px; font-family:var(--font-mono); font-size:13px; color:#c9d1d9;">$1</code>');
  
  // Replace newlines with <br>
  html = html.replace(/\n/g, "<br>");
  
  return html;
}

// Helper: Escape HTML string to prevent injection
function escapeHtml(str) {
  if (!str) return "";
  return str
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#039;");
}
