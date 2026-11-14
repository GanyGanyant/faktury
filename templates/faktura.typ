#import "@preview/tiaoma:0.3.0": qrcode

#let data = json("invoice_data.json")

#let fmt(n) = {
  let s = str(calc.round(n, digits: 2))
  let parts = s.split(".")
  let int_part = parts.at(0)
  let dec_part = if parts.len() > 1 {
    let d = parts.at(1)
    "," + if d.len() == 1 { d + "0" } else { d }
  } else { "" }
  let chars = int_part.clusters().rev()
  let grouped = ()
  for (i, c) in chars.enumerate() {
    if i > 0 and calc.rem(i, 3) == 0 and c != "-" {
      grouped.push("\u{00A0}")
    }
    grouped.push(c)
  }
  grouped.rev().join() + dec_part
}

#set page(paper: "a4", margin: (x: 1.8cm, y: 1.8cm))
#set text(font: "Libertinus Sans", size: 9.5pt, fill: rgb("#1F2937"), lang: "cs")

#grid(
  columns: (1fr, auto),
  align: (left, right),
  [
    #text(size: 20pt, weight: "bold", fill: rgb("#111827"))[FAKTURA – DAŇOVÝ DOKLAD] \
    #text(size: 11pt, fill: rgb("#4B5563"))[Evidenční číslo: *#data.invoice_number*]
  ],
  [
    #if data.supplier.iban != "" [
      #qrcode(
        "SPD*1.0*ACC:" + data.supplier.iban + "*AM:" + str(data.total_with_vat) + "*CC:CZK*X-VS:" + data.variable_symbol.replace("/", ""),
        options: (scale: 1.8)
      )
      #align(center)[#text(size: 7pt, fill: rgb("#6B7280"))[QR Platba]]
    ]
  ]
)

#v(0.8em)
#line(length: 100%, stroke: 1pt + rgb("#D1D5DB"))
#v(1em)

#grid(
  columns: (1fr, 1fr),
  gutter: 2cm,
  [
    #text(size: 8pt, weight: "bold", fill: rgb("#6B7280"))[DODAVATEL]
    #v(0.3em)
    #text(size: 11pt, weight: "bold")[#data.supplier.name] \
    #data.supplier.street \
    #data.supplier.zip #data.supplier.city \
    #v(0.5em)
    *IČO:* #data.supplier.ico \
    #if data.supplier.dic != "" [*DIČ:* #data.supplier.dic \ ]
    *Účet:* #data.supplier.bank_account \
    *IBAN:* #data.supplier.iban
  ],
  [
    #text(size: 8pt, weight: "bold", fill: rgb("#6B7280"))[ODBĚRATEL]
    #v(0.3em)
    #text(size: 11pt, weight: "bold")[#data.customer.name] \
    #data.customer.street \
    #data.customer.zip #data.customer.city \
    #v(0.5em)
    *IČO:* #data.customer.ico \
    #if data.customer.dic != "" [*DIČ:* #data.customer.dic]
  ]
)

#v(1.2em)

#block(fill: rgb("#F3F4F6"), inset: 10pt, radius: 4pt, width: 100%)[
  #grid(
    columns: (1fr, 1fr, 1fr, 1fr),
    [*Datum vystavení:* \ #data.issue_date],
    [*Datum splatnosti:* \ #data.due_date],
    [*Variabilní symbol:* \ #data.variable_symbol],
    [*Forma úhrady:* \ #data.payment_method]
  )
]

#v(1.2em)

#if data.note != "" [
  #block(
    stroke: (left: 2.5pt + rgb("#374151")),
    inset: (left: 10pt, y: 4pt),
    width: 100%
  )[
    #data.note
  ]
  #v(1.2em)
]

#table(
  columns: (1fr, auto, auto, auto, auto),
  align: (left, right, right, right, right),
  stroke: (x: none, y: 0.5pt + rgb("#E5E7EB")),
  inset: 7pt,
  fill: (_, y) => if y == 0 { rgb("#F9FAFB") },
  table.header(
    [*Popis položky*], [*Množství*], [*Cena/ks*], [*DPH*], [*Celkem*]
  ),
  ..data.items.map(item => (
    [#item.description],
    [#item.quantity #item.unit],
    [#fmt(item.unit_price) Kč],
    [#item.vat_rate %],
    [#fmt(item.total) Kč],
  )).flatten()
)

#v(1em)

#align(right)[
  #block(width: 52%)[
    #grid(
      columns: (1fr, auto),
      column-gutter: 12pt,
      row-gutter: 8pt,
      align: (left, right),
      [Celkem bez DPH:], [#fmt(data.total_without_vat) Kč],
      [DPH celkem:], [#fmt(data.total_vat) Kč],
      grid.hline(stroke: 1pt + rgb("#111827")),
      [#v(2pt) #text(size: 11pt, weight: "bold")[Celkem k úhradě:]],
      [#v(2pt) #text(size: 11pt, weight: "bold")[#fmt(data.total_with_vat) Kč]]
    )
  ]
]
