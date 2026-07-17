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
    $confirm  = $_POST['confirm'] ?? '';
    $email_val = htmlspecialchars($email);

    if (!filter_var($email, FILTER_VALIDATE_EMAIL)) {
        $error = 'Please enter a valid email address.';
    } elseif (strlen($password) < 8) {
        $error = 'Password must be at least 8 characters.';
    } elseif ($password !== $confirm) {
        $error = 'Passwords do not match.';
    } elseif (find_user_by_email($email)) {
        $error = 'An account with that email already exists. <a href="login.php">Log in?</a>';
    } else {
        $users = load_users();
        $new_user = [
            'id'             => generate_uuid(),
            'email'          => strtolower($email),
            'password_hash'  => password_hash($password, PASSWORD_BCRYPT),
            'created_at'     => date('c'),
            'name'           => '',
            'selected_model' => '',
            'platform'       => '',
            'last_login'     => date('c'),
            'is_admin'       => strtolower($email) === 'hulljessej@gmail.com',
        ];
        $users[] = $new_user;
        save_users($users);

        $_SESSION['user_id'] = $new_user['id'];
        $_SESSION['email']   = $new_user['email'];

        header('Location: dashboard.php');
        exit;
    }
}
?>
<?= page_head('Create Account') ?>

<div style="max-width:420px;margin:0 auto">
  <nav>
    <a href="index.html" class="wordmark">Ancestral Brain</a>
  </nav>

  <div class="card">
    <div class="label" style="margin-bottom:20px">Create Your Account</div>

    <?php if ($error): ?>
      <div class="error"><?= $error ?></div>
    <?php endif; ?>

    <form method="POST" novalidate>
      <div class="label">Email</div>
      <input type="email" name="email" value="<?= $email_val ?>" placeholder="your@email.com" required autofocus>

      <div class="label">Password</div>
      <input type="password" name="password" placeholder="Minimum 8 characters" required>

      <div class="label">Confirm Password</div>
      <input type="password" name="confirm" placeholder="Repeat password" required>

      <button type="submit" class="btn" style="margin-top:8px">Create Account →</button>
    </form>
  </div>

  <p style="text-align:center;color:#666;font-size:14px;margin-top:16px">
    Already have an account? <a href="login.php">Log in</a>
  </p>
</div>

</body>
</html>
