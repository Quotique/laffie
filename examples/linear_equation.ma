a in Real;
b in Real;
a is Known;
b is Known;
x is Unknown;

a*x + b = 0 => (a != 0 && x = -b/a) ||
               (a == 0 && b == 0 && x in Real) ||
               (a == 0 && b != 0 && x in EmptySet)
