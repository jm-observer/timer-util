use crate::db::Database;
use crate::error::AppError;
use crate::models::{DashboardStatsResponse, NotificationListQuery, NotificationListResponse};
use actix_web::{HttpResponse, web};

pub const DASHBOARD_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Alarm Server</title>
<style>
:root {
  --bg: #f5f5f5; --card: #fff; --border: #e0e0e0; --text: #333; --text2: #666;
  --primary: #4a90d9; --primary-hover: #357abd; --danger: #d9534f; --danger-hover: #c9302c;
  --success: #5cb85c; --warning: #f0ad4e; --info: #5bc0de;
}
* { margin: 0; padding: 0; box-sizing: border-box; }
body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; background: var(--bg); color: var(--text); }
header { background: var(--primary); color: #fff; padding: 12px 24px; display: flex; align-items: center; justify-content: space-between; }
header h1 { font-size: 18px; font-weight: 600; }
header span { font-size: 13px; opacity: .8; }
.tabs { display: flex; background: var(--card); border-bottom: 2px solid var(--border); padding: 0 24px; }
.tab { padding: 10px 20px; cursor: pointer; border-bottom: 2px solid transparent; margin-bottom: -2px; font-size: 14px; color: var(--text2); }
.tab:hover { color: var(--text); }
.tab.active { color: var(--primary); border-bottom-color: var(--primary); font-weight: 600; }
.container { max-width: 1100px; margin: 20px auto; padding: 0 20px; }
.panel { display: none; }
.panel.active { display: block; }
.stats { display: grid; grid-template-columns: repeat(auto-fit, minmax(160px, 1fr)); gap: 12px; margin-bottom: 20px; }
.stat-card { background: var(--card); border: 1px solid var(--border); border-radius: 6px; padding: 16px; text-align: center; }
.stat-card .val { font-size: 28px; font-weight: 700; color: var(--primary); }
.stat-card .label { font-size: 12px; color: var(--text2); margin-top: 4px; }
.card { background: var(--card); border: 1px solid var(--border); border-radius: 6px; padding: 16px; margin-bottom: 16px; }
.card h3 { font-size: 15px; margin-bottom: 12px; }
table { width: 100%; border-collapse: collapse; font-size: 13px; }
th, td { padding: 8px 10px; text-align: left; border-bottom: 1px solid var(--border); }
th { font-weight: 600; color: var(--text2); font-size: 12px; text-transform: uppercase; }
tr:hover { background: #f9f9f9; }
.badge { display: inline-block; padding: 2px 8px; border-radius: 10px; font-size: 11px; font-weight: 600; }
.badge-active { background: #e8f5e9; color: #2e7d32; }
.badge-completed { background: #e3f2fd; color: #1565c0; }
.badge-success { background: #e8f5e9; color: #2e7d32; }
.badge-failed { background: #ffebee; color: #c62828; }
.badge-retrying { background: #fff8e1; color: #f57f17; }
.badge-cancelled { background: #f3e5f5; color: #6a1b9a; }
.badge-cron { background: #e8eaf6; color: #283593; }
.badge-once { background: #fce4ec; color: #880e4f; }
btn, .btn { display: inline-block; padding: 6px 14px; border: none; border-radius: 4px; cursor: pointer; font-size: 13px; font-weight: 500; }
.btn-primary { background: var(--primary); color: #fff; }
.btn-primary:hover { background: var(--primary-hover); }
.btn-danger { background: var(--danger); color: #fff; }
.btn-danger:hover { background: var(--danger-hover); }
.btn-sm { padding: 3px 8px; font-size: 12px; }
.form-row { display: flex; gap: 10px; margin-bottom: 10px; align-items: flex-end; flex-wrap: wrap; }
.form-group { display: flex; flex-direction: column; }
.form-group label { font-size: 12px; color: var(--text2); margin-bottom: 3px; }
.form-group input, .form-group select, .form-group textarea { padding: 6px 10px; border: 1px solid var(--border); border-radius: 4px; font-size: 13px; font-family: inherit; }
.form-group textarea { resize: vertical; min-height: 50px; }
.toolbar { display: flex; gap: 10px; margin-bottom: 12px; align-items: center; flex-wrap: wrap; }
.pager { display: flex; gap: 8px; align-items: center; justify-content: center; margin-top: 12px; font-size: 13px; }
.pager button:disabled { opacity: .4; cursor: default; }
.empty { text-align: center; color: var(--text2); padding: 30px; font-size: 14px; }
.mono { font-family: "SF Mono", "Cascadia Code", Consolas, monospace; font-size: 12px; }
.text-truncate { max-width: 200px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; display: inline-block; vertical-align: bottom; }
.msg { padding: 8px 12px; border-radius: 4px; margin-bottom: 10px; font-size: 13px; display: none; }
.msg-ok { background: #e8f5e9; color: #2e7d32; }
.msg-err { background: #ffebee; color: #c62828; }
</style>
</head>
<body>
<header>
  <h1>Alarm Server</h1>
  <span id="hdr_stats"></span>
</header>
<div class="tabs">
  <div class="tab active" onclick="switchTab('dashboard')">Dashboard</div>
  <div class="tab" onclick="switchTab('alarms')">Alarms</div>
  <div class="tab" onclick="switchTab('notifications')">Notifications</div>
</div>
<div class="container">

<!-- ===== Dashboard ===== -->
<div id="p_dashboard" class="panel active">
  <div class="stats" id="stats_grid"></div>
  <div class="card">
    <h3>Recent Notifications</h3>
    <table><thead><tr><th>Time</th><th>Alarm</th><th>Status</th><th>HTTP</th><th>Attempt</th><th>Error</th></tr></thead>
    <tbody id="recent_tbody"></tbody></table>
  </div>
</div>

<!-- ===== Alarms ===== -->
<div id="p_alarms" class="panel">
  <div id="alarm_msg" class="msg"></div>
  <div class="card">
    <h3>Create Alarm</h3>
    <div class="form-row">
      <div class="form-group"><label>Type</label>
        <select id="f_type" onchange="toggleType()"><option value="cron">Cron</option><option value="once">Once</option></select>
      </div>
      <div class="form-group"><label>Name (optional)</label><input id="f_name" placeholder="my-alarm"></div>
      <div class="form-group" id="fg_cron"><label>Cron Expression</label><input id="f_cron" placeholder="0 30 8 * * 1-5"></div>
      <div class="form-group" id="fg_once" style="display:none"><label>Trigger At</label><input id="f_once" type="datetime-local" step="1"></div>
      <div class="form-group"><label>Callback URL</label><input id="f_url" placeholder="http://example.com/hook" style="min-width:220px"></div>
    </div>
    <div class="form-row">
      <div class="form-group" style="flex:1"><label>Callback Body (JSON, optional)</label><textarea id="f_body" placeholder='{"key":"value"}'></textarea></div>
      <div class="form-group"><label>&nbsp;</label><button class="btn btn-primary" onclick="createAlarm()">Create</button></div>
    </div>
  </div>
  <div class="card">
    <div class="toolbar">
      <h3 style="margin:0">Alarm List</h3>
      <select id="alarm_filter" onchange="loadAlarms()"><option value="">All</option><option value="active">Active</option><option value="completed">Completed</option></select>
      <button class="btn btn-primary btn-sm" onclick="loadAlarms()">Refresh</button>
    </div>
    <table><thead><tr><th>Name</th><th>Type</th><th>Schedule</th><th>Callback</th><th>Status</th><th>Next Fire</th><th>Created</th><th></th></tr></thead>
    <tbody id="alarm_tbody"></tbody></table>
    <div id="alarm_empty" class="empty" style="display:none">No alarms</div>
  </div>
</div>

<!-- ===== Notifications ===== -->
<div id="p_notifications" class="panel">
  <div class="card">
    <div class="toolbar">
      <h3 style="margin:0">Notification Logs</h3>
      <div class="form-group"><label>Status</label>
        <select id="notif_status" onchange="loadNotifs(1)"><option value="">All</option><option value="success">Success</option><option value="failed">Failed</option><option value="retrying">Retrying</option><option value="cancelled">Cancelled</option></select>
      </div>
      <div class="form-group"><label>Alarm ID</label><input id="notif_aid" placeholder="filter..." style="width:140px" onchange="loadNotifs(1)"></div>
      <button class="btn btn-primary btn-sm" onclick="loadNotifs(1)">Refresh</button>
    </div>
    <table><thead><tr><th>Time</th><th>Alarm</th><th>URL</th><th>Status</th><th>HTTP</th><th>Attempt</th><th>Error</th></tr></thead>
    <tbody id="notif_tbody"></tbody></table>
    <div id="notif_empty" class="empty" style="display:none">No notifications</div>
    <div class="pager">
      <button class="btn btn-sm" id="notif_prev" onclick="loadNotifs(notifPage-1)">Prev</button>
      <span id="notif_info"></span>
      <button class="btn btn-sm" id="notif_next" onclick="loadNotifs(notifPage+1)">Next</button>
    </div>
  </div>
</div>

</div>
<script>
let notifPage = 1;
const PER_PAGE = 20;

function switchTab(name) {
  document.querySelectorAll('.tab').forEach((t,i) => t.classList.toggle('active', t.textContent.trim().toLowerCase() === name));
  document.querySelectorAll('.panel').forEach(p => p.classList.toggle('active', p.id === 'p_'+name));
  if (name === 'dashboard') loadDashboard();
  else if (name === 'alarms') loadAlarms();
  else if (name === 'notifications') loadNotifs(1);
}

function badge(text, type) {
  return '<span class="badge badge-'+(type||text)+'">'+esc(text)+'</span>';
}
function esc(s) { const d=document.createElement('div'); d.textContent=s||''; return d.innerHTML; }
function shortId(id) { return id ? id.substring(0,8) : ''; }

function showMsg(id, ok, text) {
  const el = document.getElementById(id);
  el.className = 'msg ' + (ok ? 'msg-ok' : 'msg-err');
  el.textContent = text;
  el.style.display = 'block';
  setTimeout(() => el.style.display='none', 4000);
}

async function loadDashboard() {
  try {
    const r = await fetch('/api/dashboard/stats');
    const d = await r.json();
    const items = [
      {v: d.total_alarms, l: 'Total Alarms'},
      {v: d.active_alarms, l: 'Active'},
      {v: d.completed_alarms, l: 'Completed'},
      {v: d.total_notifications, l: 'Notifications'},
      {v: d.successful_notifications, l: 'Succeeded'},
      {v: d.failed_notifications, l: 'Failed'},
    ];
    document.getElementById('stats_grid').innerHTML = items.map(i =>
      '<div class="stat-card"><div class="val">'+i.v+'</div><div class="label">'+i.l+'</div></div>'
    ).join('');
    document.getElementById('hdr_stats').textContent = d.active_alarms+' active / '+d.total_notifications+' notifications';
    const tbody = document.getElementById('recent_tbody');
    if (d.recent_notifications && d.recent_notifications.length) {
      tbody.innerHTML = d.recent_notifications.map(n =>
        '<tr><td class="mono">'+esc(n.triggered_at)+'</td><td>'+esc(n.alarm_name||shortId(n.alarm_id))+'</td><td>'+badge(n.status)+'</td><td>'+(n.http_status||'-')+'</td><td>'+n.attempt+'</td><td class="text-truncate">'+esc(n.error_message||'')+'</td></tr>'
      ).join('');
    } else {
      tbody.innerHTML = '<tr><td colspan="6" style="text-align:center;color:#999">No recent notifications</td></tr>';
    }
  } catch(e) { console.error(e); }
}

function toggleType() {
  const t = document.getElementById('f_type').value;
  document.getElementById('fg_cron').style.display = t==='cron' ? '' : 'none';
  document.getElementById('fg_once').style.display = t==='once' ? '' : 'none';
}

async function createAlarm() {
  const type = document.getElementById('f_type').value;
  const name = document.getElementById('f_name').value.trim();
  const url = document.getElementById('f_url').value.trim();
  if (!url) { showMsg('alarm_msg', false, 'Callback URL is required'); return; }

  const body = {alarm_type: type, callback_url: url};
  if (name) body.name = name;

  if (type === 'cron') {
    const cron = document.getElementById('f_cron').value.trim();
    if (!cron) { showMsg('alarm_msg', false, 'Cron expression is required'); return; }
    body.cron_expr = cron;
  } else {
    const raw = document.getElementById('f_once').value;
    if (!raw) { showMsg('alarm_msg', false, 'Trigger time is required'); return; }
    body.once_at = raw.replace(' ','T');
    if (body.once_at.length === 16) body.once_at += ':00';
  }

  const cbody = document.getElementById('f_body').value.trim();
  if (cbody) {
    try { body.callback_body = JSON.parse(cbody); }
    catch(e) { showMsg('alarm_msg', false, 'Invalid JSON in callback body'); return; }
  }

  try {
    const r = await fetch('/api/alarms', {method:'POST', headers:{'Content-Type':'application/json'}, body: JSON.stringify(body)});
    if (r.ok) {
      showMsg('alarm_msg', true, 'Alarm created');
      document.getElementById('f_name').value = '';
      document.getElementById('f_cron').value = '';
      document.getElementById('f_once').value = '';
      document.getElementById('f_url').value = '';
      document.getElementById('f_body').value = '';
      loadAlarms();
    } else {
      const e = await r.json().catch(()=>({}));
      showMsg('alarm_msg', false, e.error || 'Failed: HTTP '+r.status);
    }
  } catch(e) { showMsg('alarm_msg', false, 'Request failed: '+e.message); }
}

async function loadAlarms() {
  const status = document.getElementById('alarm_filter').value;
  const url = status ? '/api/alarms?status='+status : '/api/alarms';
  try {
    const r = await fetch(url);
    const d = await r.json();
    const tbody = document.getElementById('alarm_tbody');
    const empty = document.getElementById('alarm_empty');
    if (!d.alarms || d.alarms.length === 0) {
      tbody.innerHTML = '';
      empty.style.display = 'block';
      return;
    }
    empty.style.display = 'none';
    tbody.innerHTML = d.alarms.map(a =>
      '<tr>' +
      '<td>'+esc(a.name || shortId(a.id))+'</td>' +
      '<td>'+badge(a.alarm_type)+'</td>' +
      '<td class="mono">'+esc(a.alarm_type==='cron' ? a.cron_expr : a.once_at)+'</td>' +
      '<td class="mono text-truncate">'+esc(a.callback_url)+'</td>' +
      '<td>'+badge(a.status)+'</td>' +
      '<td class="mono">'+(a.next_fire_at || '-')+'</td>' +
      '<td class="mono">'+esc(a.created_at)+'</td>' +
      '<td><button class="btn btn-danger btn-sm" onclick="deleteAlarm(\''+esc(a.id)+'\')">Delete</button></td>' +
      '</tr>'
    ).join('');
  } catch(e) { console.error(e); }
}

async function deleteAlarm(id) {
  if (!confirm('Delete this alarm?')) return;
  try {
    const r = await fetch('/api/alarms/'+id, {method:'DELETE'});
    if (r.ok || r.status === 204) {
      showMsg('alarm_msg', true, 'Alarm deleted');
      loadAlarms();
    } else {
      showMsg('alarm_msg', false, 'Delete failed: HTTP '+r.status);
    }
  } catch(e) { showMsg('alarm_msg', false, 'Request failed'); }
}

async function loadNotifs(page) {
  if (page < 1) page = 1;
  notifPage = page;
  const status = document.getElementById('notif_status').value;
  const aid = document.getElementById('notif_aid').value.trim();
  let url = '/api/notifications?page='+page+'&per_page='+PER_PAGE;
  if (status) url += '&status='+status;
  if (aid) url += '&alarm_id='+aid;
  try {
    const r = await fetch(url);
    const d = await r.json();
    const tbody = document.getElementById('notif_tbody');
    const empty = document.getElementById('notif_empty');
    if (!d.notifications || d.notifications.length === 0) {
      tbody.innerHTML = '';
      empty.style.display = 'block';
    } else {
      empty.style.display = 'none';
      tbody.innerHTML = d.notifications.map(n =>
        '<tr>' +
        '<td class="mono">'+esc(n.triggered_at)+'</td>' +
        '<td>'+esc(n.alarm_name||shortId(n.alarm_id))+'</td>' +
        '<td class="mono text-truncate">'+esc(n.callback_url)+'</td>' +
        '<td>'+badge(n.status)+'</td>' +
        '<td>'+(n.http_status||'-')+'</td>' +
        '<td>'+n.attempt+'</td>' +
        '<td class="text-truncate">'+esc(n.error_message||'')+'</td>' +
        '</tr>'
      ).join('');
    }
    const total = d.total || 0;
    const totalPages = Math.ceil(total / PER_PAGE) || 1;
    document.getElementById('notif_info').textContent = page + ' / ' + totalPages + '  (' + total + ')';
    document.getElementById('notif_prev').disabled = page <= 1;
    document.getElementById('notif_next').disabled = page >= totalPages;
  } catch(e) { console.error(e); }
}

window.onload = function() { loadDashboard(); setInterval(loadDashboard, 30000); };
</script>
</body>
</html>"#;

pub async fn dashboard_page(_db: web::Data<Database>) -> Result<HttpResponse, AppError> {
    Ok(HttpResponse::Ok()
        .content_type("text/html")
        .body(DASHBOARD_HTML))
}

pub async fn dashboard_stats(db: web::Data<Database>) -> Result<HttpResponse, AppError> {
    let total_alarms = db.count_alarms(None).map_err(AppError::from)?;
    let active_alarms = db.count_by_status("active").map_err(AppError::from)?;
    let completed_alarms = db.count_by_status("completed").map_err(AppError::from)?;
    let notif_stats = db.notification_stats().map_err(AppError::from)?;
    let recent = {
        let (list, _) = db
            .list_notification_logs(None, None, 1, 5)
            .map_err(AppError::from)?;
        list
    };
    let resp = DashboardStatsResponse {
        total_alarms,
        active_alarms,
        completed_alarms,
        total_notifications: notif_stats.total,
        successful_notifications: notif_stats.success,
        failed_notifications: notif_stats.failed,
        recent_notifications: recent,
    };
    Ok(HttpResponse::Ok().json(resp))
}

pub async fn list_notifications(
    query: web::Query<NotificationListQuery>,
    db: web::Data<Database>,
) -> Result<HttpResponse, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).clamp(1, 100);
    let (notifications, total) = db
        .list_notification_logs(
            query.alarm_id.as_deref(),
            query.status.as_deref(),
            page,
            per_page,
        )
        .map_err(AppError::from)?;
    let resp = NotificationListResponse {
        notifications,
        total,
    };
    Ok(HttpResponse::Ok().json(resp))
}
