# Zpěvník pro Android

> Note: This section is in the Czech language as it is only relevant to users of a specific Czech-only [Android app](http://karel-hovorka.eu/zpevnik/).

Bard umí generovat zpěvníky pro aplikaci [_Zpěvník_](http://karel-hovorka.eu/zpevnik/) p. Karla Hovorky.

Tyto zpěvníky jsou ve formátu XML a vytvoříme je přidáním dalšího výstupu (`[[output]]`) v souboru `bard.toml`.
Aby se odlišily od obecného [XML výstupu](./json-and-xml.md), je potřeba explicitně nastavit formát výstupu na `hovorka`. Například:

```toml
[[output]]
file = "songbook.hovorka.xml"
format = "hovorka"
```

Vygenerovaný soubor `songbook.hovorka.xml` následně můžeme zkopírovat do telefonu nebo tabletu s Androidem a importovat ho v aplikací _Zpěvník_.
