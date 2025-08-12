#include <iostream>

void goal() {
  std::string flag = "flag{flag_is_here}";
  std::cout << flag << std::endl;
}

int main() {
  ${{user_input}}
  int x = 1;
  if (x == 0) { goal(); }
  return 0;
}