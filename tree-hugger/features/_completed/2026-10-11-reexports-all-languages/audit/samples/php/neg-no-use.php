<?php
namespace App;
class Bar {}
function f() { return \Foo\Bar::class; }
$x = new \Foo\Bar();
trait T {} class U { use T; }
$cl = function() use ($x) { return $x; };
