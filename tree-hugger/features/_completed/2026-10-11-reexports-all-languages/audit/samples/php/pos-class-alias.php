<?php
class_alias('Foo\Bar', 'Baz');
class_alias(\Foo\Bar::class, 'Qux', false);
