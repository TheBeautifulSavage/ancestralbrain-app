<?php
$sessions_dir = __DIR__ . '/data/sessions';
if (!is_dir($sessions_dir)) mkdir($sessions_dir, 0700, true);
session_save_path($sessions_dir);
session_start();

require_once __DIR__ . '/_helpers.php';

// Already logged in → go to dashboard
if (!empty($_SESSION['user_id'])) {
    header('Location: dashboard.php');
    exit;
}

$error = '';
$email_val = '';

if ($_SERVER['REQUEST_METHOD'] === 'POST') {
    $email    = trim($_POST['email'] ?? '');
    $password = $_POST['password'] ?? '';
    $email_val = htmlspecialchars($email);

    $user = find_user_by_email($email);
    if ($user && password_verify($password, $user['password_hash'])) {
        // Update last_login
        $users = load_users();
        foreach ($users as &$u) {
            if ($u['id'] === $user['id']) {
                $u['last_login'] = date('c');
                break;
            }
        }
        unset($u);
        save_users($users);

        $_SESSION['user_id'] = $user['id'];
        $_SESSION['email']   = $user['email'];

        header('Location: dashboard.php');
        exit;
    } else {
        $error = 'Invalid email or password.';
    }
}
?>
<?= page_head('Log In') ?>

<div style="max-width:420px;margin:0 auto">
  <nav>
    <a href="index.html" class="wordmark">Ancestral Brain</a>
  </nav>

  <div class="card">
    <div class="label" style="margin-bottom:20px">Log In to Your Account</div>

    <?php if ($error): ?>
      <div class="error"><?= htmlspecialchars($error) ?></div>
    <?php endif; ?>

    <form method="POST" novalidate>
      <div class="label">Email</div>
      <input type="email" name="email" value="<?= $email_val ?>" placeholder="your@email.com" required autofocus>

      <div class="label">Password</div>
      <input type="password" name="password" placeholder="Your password" required>

      <button type="submit" class="btn" style="margin-top:8px">Log In →</button>
    </form>
  </div>

  <p style="text-align:center;color:#666;font-size:14px;margin-top:16px">
    Don't have an account? <a href="register.php">Create one</a>
  </p>
</div>

</body>
</html>
