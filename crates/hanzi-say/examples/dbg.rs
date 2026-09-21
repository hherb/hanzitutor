fn main() {
    use hanzi_say::phonetics::*;
    for p in [("qing3","xing1"),("jin4","xing1"),("jin4","ji4"),("ke4","e4"),("ke3","ke3")] {
        let a: std::collections::HashSet<_> = parse(p.0).into_iter().collect();
        let b: std::collections::HashSet<_> = parse(p.1).into_iter().collect();
        let sa=parse(p.0).unwrap(); let sb=parse(p.1).unwrap();
        println!("{} ({},{}) vs {} ({},{}): {:?}",
            p.0, sa.initial, sa.final_, p.1, sb.initial, sb.final_, compare(&a,&b));
    }
}
