<?php
// Multi-file fixture: PHP.

class Account
{
    private int $balance = 0;

    public function deposit(int $amount): void
    {
        $this->balance += $amount;
    }

    public static function open(): Account
    {
        return new Account();
    }
}

function open_account(): Account
{
    return Account::open();
}
