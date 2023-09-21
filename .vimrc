map <C-S-F10> :wa \| bel term bash -c "RUST_BACKTRACE=full cargo test --tests"<CR>
map <S-F10> :wa \| bel term bash -c "RUST_BACKTRACE=full cargo test --lib"<CR>
