<?php
// waitlist.php — Ancestral Brain waitlist
// Stores to JSON flat-file (SQLite unavailable on Hostinger shared hosting)
// CORS locked to ancestralbrain.com

header('Content-Type: application/json');
header('Access-Control-Allow-Origin: https://ancestralbrain.com');
header('Access-Control-Allow-Methods: POST, OPTIONS');
header('Access-Control-Allow-Headers: Content-Type');

if ($_SERVER['REQUEST_METHOD'] === 'OPTIONS') {
    http_response_code(204);
    exit;
}

if ($_SERVER['REQUEST_METHOD'] !== 'POST') {
    http_response_code(405);
    echo json_encode(['ok' => false, 'error' => 'Method not allowed']);
    exit;
}

// --- Input ---
$raw  = file_get_contents('php://input');
$data = json_decode($raw, true) ?: $_POST;

$email = isset($data['email']) ? trim(strtolower($data['email'])) : '';
$name  = isset($data['name'])  ? trim($data['name'])              : '';

if (!filter_var($email, FILTER_VALIDATE_EMAIL)) {
    http_response_code(400);
    echo json_encode(['ok' => false, 'error' => 'Invalid email']);
    exit;
}

if (strlen($email) > 254 || strlen($name) > 120) {
    http_response_code(400);
    echo json_encode(['ok' => false, 'error' => 'Input too long']);
    exit;
}

// --- Storage: JSON flat-file outside public_html ---
$data_dir  = dirname(__DIR__) . '/private';
$data_file = $data_dir . '/ancestralbrain_waitlist.json';

if (!is_dir($data_dir)) {
    mkdir($data_dir, 0750, true);
}

// Exclusive lock to prevent race conditions
$lock = fopen($data_file . '.lock', 'c');
if (!flock($lock, LOCK_EX)) {
    http_response_code(500);
    echo json_encode(['ok' => false, 'error' => 'Server error']);
    exit;
}

try {
    $entries = [];
    if (file_exists($data_file)) {
        $entries = json_decode(file_get_contents($data_file), true) ?: [];
    }

    // Check for duplicate
    foreach ($entries as $e) {
        if ($e['email'] === $email) {
            flock($lock, LOCK_UN);
            fclose($lock);
            echo json_encode(['ok' => true, 'message' => 'Already registered!']);
            exit;
        }
    }

    $entries[] = [
        'email'   => $email,
        'name'    => $name,
        'source'  => 'web',
        'ip'      => $_SERVER['REMOTE_ADDR'] ?? '',
        'created' => date('c'),
    ];

    file_put_contents($data_file, json_encode($entries, JSON_PRETTY_PRINT));

} finally {
    flock($lock, LOCK_UN);
    fclose($lock);
}

echo json_encode(['ok' => true, 'message' => "You're on the list!"]);
