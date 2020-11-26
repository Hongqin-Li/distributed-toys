set -o errexit

nworkers=10

txt_dir=../MIT-6.824/src/main
ntxt=$(find $txt_dir -name "*.txt" | wc -l)
nreduce=20

echo $ntxt

export RUST_LOG=sequential=trace,worker=trace,master=trace
SEQUENTIAL="./target/debug/sequential --nmap $ntxt --nreduce $nreduce $txt_dir/*.txt"
MASTER="./target/debug/master --nmap $ntxt --nreduce $nreduce --port 9999 $txt_dir/*.txt"
WORKER="./target/debug/worker --server=127.0.0.1:9999"

rm -f target/mr*

cargo build

${SEQUENTIAL}

ps=""
for i in $(seq 1 $nworkers); do
  ${WORKER} &
  ps="$ps $!"
done

${MASTER}

cd target

mrs_out=$(find ./ -name "mrs-*")
for mrs in $mrs_out; do
  mr="mr${mrs:5}"
  d=`diff $mr $mrs`
  if [ ! -z "$d" ]; then
    echo "diff between '$mr' and '$mrs'"
    exit 1
  fi
  echo "- checked: $mr"
done

cd ..


nfails=0
for p in $ps; do
  echo "- waiting $p"
  wait $p || nfails=$(echo "$nfails+1" | bc)
done

echo "- nfails: $nfails"

if [ $nfails -ne 0 ]; then
  echo "- ERROR: some worker failed"
  exit 1
fi

echo "script end"
