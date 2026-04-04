// Dashboard module providing HTML UI and API endpoints

pub const DASHBOARD_HTML: &str = r#"<!DOCTYPE html>
<html lang=\"en\">
<head>
<meta charset=\"UTF-8\">
<title>Alarm Server Dashboard</title>
<link rel=\"stylesheet\" href=\"https://cdn.simplecss.org/simple.min.css\">
<script>
async function loadStats() {
    const resp = await fetch('/api/dashboard/stats');
    const data = await resp.json();
    document.getElementById('total_alarms').textContent = data.total_alarms;
    document.getElementById('active_alarms').textContent = data.active_alarms;
    document.getElementById('completed_alarms').textContent = data.completed_alarms;
    document.getElementById('total_notifications').textContent = data.total_notifications;
    document.getElementById('successful_notifications').textContent = data.successful_notifications;
    document.getElementById('failed_notifications').textContent = data.failed_notifications;
}
async function loadNotifications(page=1) {
    const resp = await fetch(`/api/notifications?page=${page}`);
    const data = await resp.json();
    const tbody = document.getElementById('notif_body');
    tbody.innerHTML = '';
    data.notifications.forEach(n => {
        const tr = document.createElement('tr');
        tr.innerHTML = `<td>${n.triggered_at}</td><td>${n.alarm_name}</td><td>${n.status}</td><td>${n.http_status||''}</td><td>${n.attempt}</td><td>${n.error_message||''}</td>`;
        tbody.appendChild(tr);
    });
    document.getElementById('prev_page').disabled = page <= 1;
    document.getElementById('next_page').disabled = (page * 20) >= data.total;
    document.getElementById('prev_page').onclick = () => loadNotifications(page-1);
    document.getElementById('next_page').onclick = () => loadNotifications(page+1);
}
window.onload = function(){ loadStats(); loadNotifications(); setInterval(loadStats, 30000); };
</script>
</head>
<body>
<h1>Alarm Server Dashboard</h1>
<div>
<p>Total Alarms: <span id=\"total_alarms\"></span></p>
<p>Active Alarms: <span id=\"active_alarms\"></span></p>
<p>Completed Alarms: <span id=\"completed_alarms\"></span></p>
<p>Total Notifications: <span id=\"total_notifications\"></span></p>
<p>Successful Notifications: <span id=\"successful_notifications\"></span></p>
<p>Failed Notifications: <span id=\"failed_notifications\"></span></p>
</div>
<h2>Recent Notifications</h2>
<table>
<thead><tr><th>Triggered At</th><th>Alarm</th><th>Status</th><th>HTTP</th><th>Attempt</th><th>Error</th></tr></thead>
<tbody id=\"notif_body\"></tbody>
</table>
<button id=\"prev_page\">Previous</button>
<button id=\"next_page\">Next</button>
</body>
</html>"#;

use actix_web::{web, HttpResponse};
use crate::models::{DashboardStatsResponse, NotificationListResponse, NotificationListQuery};
use crate::db::Database;
use crate::error::AppError;

pub async fn dashboard_page(_db: web::Data<Database>) -> Result<HttpResponse, AppError> {
    Ok(HttpResponse::Ok().content_type("text/html").body(DASHBOARD_HTML))
}

pub async fn dashboard_stats(db: web::Data<Database>) -> Result<HttpResponse, AppError> {
    let total_alarms = db.list_alarms(None).map_err(AppError::from)?.len();
    let active_alarms = db.count_by_status("active").map_err(AppError::from)?;
    let completed_alarms = db.count_by_status("completed").map_err(AppError::from)?;
    let notif_stats = db.notification_stats().map_err(AppError::from)?;
    let recent = {
        let (list, _) = db.list_notification_logs(None, None, 1, 5).map_err(AppError::from)?;
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
    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(20);
    let (notifications, total) = db.list_notification_logs(
        query.alarm_id.as_deref(),
        query.status.as_deref(),
        page,
        per_page,
    ).map_err(AppError::from)?;
    let resp = NotificationListResponse { notifications, total };
    Ok(HttpResponse::Ok().json(resp))
}
