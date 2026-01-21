<?php
// Test fixture for PHP import extraction

namespace App\Test;

// Simple use
use App\Models\User;

// Aliased use
use App\Services\AuthService as Auth;

// Function import
use function App\Helpers\format_date;

// Constant import
use const App\Config\VERSION;

class ImportsTest
{
    public function test(): void
    {
        $user = new User();
        $auth = new Auth();
    }
}
