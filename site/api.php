<?php
$sessions_dir = __DIR__ . '/data/sessions';
if (!is_dir($sessions_dir)) mkdir($sessions_dir, 0700, true);
session_save_path($sessions_dir);
session_start();

require_once __DIR__ . '/_helpers.php';

header('Content-Type: application/json');

$action = $_GET['action'] ?? ($_POST['action'] ?? '');

// ── Check session ──────────────────────────────────────────────────────────
if ($action === 'check_session') {
    $user = current_user();
    echo json_encode([
        'logged_in' => (bool)$user,
        'email'     => $user ? $user['email'] : null,
    ]);
    exit;
}

// ── Waitlist (keep for legacy subscribers) ────────────────────────────────
if ($_SERVER['REQUEST_METHOD'] === 'POST' && !$action) {
    $body = json_decode(file_get_contents('php://input'), true);
    $email = trim($body['email'] ?? '');
    $name  = trim($body['name']  ?? '');

    if (!filter_var($email, FILTER_VALIDATE_EMAIL)) {
        http_response_code(400);
        echo json_encode(['ok' => false, 'error' => 'Invalid email address.']);
        exit;
    }

    $waitlist_file = __DIR__ . '/data/waitlist.json';
    $waitlist = [];
    if (file_exists($waitlist_file)) {
        $waitlist = json_decode(file_get_contents($waitlist_file), true) ?: [];
    }

    // Check for duplicate
    foreach ($waitlist as $entry) {
        if (strtolower($entry['email']) === strtolower($email)) {
            echo json_encode(['ok' => true, 'message' => "You're already on the list! We'll be in touch."]);
            exit;
        }
    }

    $waitlist[] = [
        'email'      => strtolower($email),
        'name'       => $name,
        'created_at' => date('c'),
    ];

    if (!is_dir(data_dir())) mkdir(data_dir(), 0700, true);
    file_put_contents($waitlist_file, json_encode($waitlist, JSON_PRETTY_PRINT));

    echo json_encode([
        'ok'      => true,
        'message' => "You're on the list! We'll send download instructions when alpha opens.",
    ]);
    exit;
}

http_response_code(400);
echo json_encode(['ok' => false, 'error' => 'Unknown action']);
