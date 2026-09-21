fn main() {
    for t in ["谢谢你！不客气。", "你好，很高兴认识你。", "对不起。没关系。", "老师，您好！"] {
        let c = hanzi_say::chunk_text(t);
        println!("{:?} -> {} chunk(s): {:?}", t, c.len(), c);
    }
}
