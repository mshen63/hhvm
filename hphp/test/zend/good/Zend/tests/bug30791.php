<?php

function my_error_handler($errno, $errstr, $errfile, $errline) {
    var_dump($errstr);
}

class a
{
   public $a = 4;
   function __call($a,$b) {
       return "unknown method";
   }
}
<<__EntryPoint>> function main() {
set_error_handler('my_error_handler');

$b = new a;
echo $b,"\n";
$c = unserialize(serialize($b));
echo $c,"\n";
var_dump($c);
}
