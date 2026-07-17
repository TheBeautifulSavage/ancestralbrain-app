<?php
$sessions_dir = __DIR__ . '/data/sessions';
if (!is_dir($sessions_dir)) mkdir($sessions_dir, 0700, true);
session_save_path($sessions_dir);
session_start();
session_destroy();
header('Location: index.html');
exit;
