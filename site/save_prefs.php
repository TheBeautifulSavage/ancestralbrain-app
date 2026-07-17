<?php
$sessions_dir = __DIR__ . '/data/sessions';
if (!is_dir($sessions_dir)) mkdir($sessions_dir, 0700, true);
session_save_path($sessions_dir);
session_start();

require_once __DIR__ . '/_helpers.php';

header('Content-Type: application/json');

$user = current_user();
if (!$user) {
    echo json_encode(['ok' => false, 'error' => 'Not logged in']);
    exit;
}

$body = json_decode(file_get_contents('php://input'), true);
$selected_model = trim($body['selected_model'] ?? '');
$platform       = trim($body['platform'] ?? '');

$allowed_models = ['llama3.2:1b', 'llama3.2:3b', 'llama3.1:8b'];
if ($selected_model && !in_array($selected_model, $allowed_models)) {
    echo json_encode(['ok' => false, 'error' => 'Invalid model']);
    exit;
}

$users = load_users();
foreach ($users as &$u) {
    if ($u['id'] === $user['id']) {
        if ($selected_model) $u['selected_model'] = $selected_model;
        if ($platform)       $u['platform']       = $platform;
        break;
    }
}
unset($u);
save_users($users);

echo json_encode(['ok' => true]);
