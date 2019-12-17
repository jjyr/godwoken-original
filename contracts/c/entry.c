int contract_entry();

int ckb_debug(const char*);

int main() {
  return contract_entry();
}

void print_dbg() {
  ckb_debug("this is from ckb debug");
}


