<?php
function data_dir() {
    return __DIR__ . '/data';
}
function users_file() {
    return data_dir() . '/users.json';
}
function load_users() {
    if (!file_exists(users_file())) return [];
    return json_decode(file_get_contents(users_file()), true) ?: [];
}
function save_users($users) {
    if (!is_dir(data_dir())) mkdir(data_dir(), 0700, true);
    file_put_contents(users_file(), json_encode($users, JSON_PRETTY_PRINT));
}
function find_user_by_email($email) {
    foreach (load_users() as $u) {
        if (strtolower($u['email']) === strtolower($email)) return $u;
    }
    return null;
}
function current_user() {
    if (empty($_SESSION['user_id'])) return null;
    foreach (load_users() as $u) {
        if ($u['id'] === $_SESSION['user_id']) return $u;
    }
    return null;
}
function generate_uuid() {
    return sprintf(
        '%04x%04x-%04x-%04x-%04x-%04x%04x%04x',
        mt_rand(0, 0xffff), mt_rand(0, 0xffff),
        mt_rand(0, 0xffff),
        mt_rand(0, 0x0fff) | 0x4000,
        mt_rand(0, 0x3fff) | 0x8000,
        mt_rand(0, 0xffff), mt_rand(0, 0xffff), mt_rand(0, 0xffff)
    );
}
function page_head($title) {
    return <<<HTML
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{$title} — Ancestral Brain</title>
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;600;800&display=swap" rel="stylesheet">
  <style>
    *,*::before,*::after{box-sizing:border-box;margin:0;padding:0}
    :root{--bg:#0a0a0a;--amber:#f5a623;--white:#fff;--gray:#999;--border:rgba(255,255,255,0.08);--card:#111}
    body{background:var(--bg);color:var(--white);font-family:'Inter',sans-serif;min-height:100vh;padding:40px 20px}
    a{color:var(--amber);text-decoration:none}
    input{width:100%;padding:12px 14px;background:#1a1a1a;border:1px solid var(--border);border-radius:6px;color:#fff;font-size:15px;font-family:inherit;margin-bottom:12px}
    input:focus{outline:none;border-color:var(--amber)}
    .btn{display:block;width:100%;padding:13px 20px;background:var(--amber);color:#000;font-weight:700;font-size:15px;border:none;border-radius:6px;cursor:pointer;font-family:inherit;text-align:center;text-decoration:none}
    .btn:hover{background:#e09610}
    .btn-ghost{background:transparent;border:1px solid var(--border);color:#fff}
    .btn-ghost:hover{border-color:var(--amber);color:var(--amber)}
    .card{background:var(--card);border:1px solid var(--border);border-radius:10px;padding:24px;margin-bottom:20px}
    .error{color:#ff6b6b;font-size:14px;margin-bottom:12px}
    .label{font-size:12px;font-weight:600;color:var(--amber);letter-spacing:.08em;text-transform:uppercase;margin-bottom:8px}
    nav{display:flex;justify-content:space-between;align-items:center;padding:0 0 32px;border-bottom:1px solid var(--border);margin-bottom:32px}
    .wordmark{font-weight:800;font-size:20px;color:#fff;text-decoration:none}
  </style>
</head>
<body>
HTML;
}
