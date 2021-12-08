
cd build
git clone https://github.com/nervosnetwork/ckb-standalone-debugger.git
cd ckb-standalone-debugger/
git checkout f2df7e54e0cfabadf77cabeb09d202635412f1c8
if (( $? == 0 ))
then
    echo "succcess"
else
    exit 1
fi

git am ../../ckb-debugger.patch
if (( $? == 0 ))
then
    echo "succcess"
else
    exit 1
fi

cd bins
cargo build --release
if (( $? == 0 ))
then
    echo "succcess"
else
    exit 1
fi

rm -f target/release/deps/ckb_debugger-*.d
cp target/release/deps/ckb_debugger-* ../../
cd ../../
for x in ckb_debugger-*; do
    mv "$x" "ckb-debugger-bins"
done