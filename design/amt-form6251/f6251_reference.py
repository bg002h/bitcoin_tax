"""Form 6251 (2024), transcribed line by line from PART_III.md. Variable names ARE line numbers."""
from decimal import Decimal as D

BR = {  # 2024 ordinary brackets — §1(j)(2), Rev. Proc. 2023-34. (upper_bound, rate); None = no ceiling.
 #
 # ★ PROVENANCE, stated because it bounds what this file can witness. The AMT-specific constants below
 # (EXM/PHASE/BP28/SUB/L19/L25/KICK_*) are transcribed from the Form 6251 PDF and i6251 — primary
 # sources. These ORDINARY brackets and STD are not printed on Form 6251, and are cross-checked against
 # `btctax-adapters/src/tax_tables.rs` (itself encoded verbatim from the Rev. Procs.) — i.e. against
 # OUR OWN table, so they are NOT independent. They feed form line 10 (the regular tax), which
 # OpenTaxSolver witnesses independently in `verify_f6251.py`; that is what actually guards them.
 'mfj':[(D(23200),D('.10')),(D(94300),D('.12')),(D(201050),D('.22')),(D(383900),D('.24')),(D(487450),D('.32')),(D(731200),D('.35')),(None,D('.37'))],
 'mfs':[(D(11600),D('.10')),(D(47150),D('.12')),(D(100525),D('.22')),(D(191950),D('.24')),(D(243725),D('.32')),(D(365600),D('.35')),(None,D('.37'))],
 'single':[(D(11600),D('.10')),(D(47150),D('.12')),(D(100525),D('.22')),(D(191950),D('.24')),(D(243725),D('.32')),(D(609350),D('.35')),(None,D('.37'))],
 'hoh':[(D(16550),D('.10')),(D(63100),D('.12')),(D(100500),D('.22')),(D(191950),D('.24')),(D(243700),D('.32')),(D(609350),D('.35')),(None,D('.37'))]}
STD   = {'mfj':D(29200),'mfs':D(14600),'single':D(14600),'hoh':D(21900)}
# ── Everything below is transcribed from the PRIMARY source, quoted where it is not obvious. ──
# i6251 p.1: "The exemption amount on Form 6251, line 5, has increased to $85,700 ($133,300 if married
# filing jointly or qualifying surviving spouse; $66,650 if married filing separately)."
EXM   = {'mfj':D(133300),'mfs':D(66650),'single':D(85700),'hoh':D(85700)}    # form line 5
# i6251 p.1: "the amount used to determine the phaseout of your exemption has increased to $609,350
# ($1,218,700 if married filing jointly or qualifying surviving spouse)."
PHASE = {'mfj':D(1218700),'mfs':D(609350),'single':D(609350),'hoh':D(609350)}  # form line 5
# Form 6251 lines 18/39: "$232,600 or less ($116,300 or less if married filing separately)".
BP28  = {'mfj':D(232600),'mfs':D(116300),'single':D(232600),'hoh':D(232600)}
SUB   = {'mfj':D(4652),'mfs':D(2326),'single':D(4652),'hoh':D(4652)}
# Form 6251 line 19 (0% band top): "$94,050 if married filing jointly or qualifying surviving spouse,
# $47,025 if single or married filing separately, or $63,000 if head of household."
L19   = {'mfj':D(94050),'mfs':D(47025),'single':D(47025),'hoh':D(63000)}
# Form 6251 line 25 (15% band top): "$518,900 if single, $291,850 if married filing separately,
# $583,750 if married filing jointly or qualifying surviving spouse, or $551,350 if head of household."
L25   = {'mfj':D(583750),'mfs':D(291850),'single':D(518900),'hoh':D(551350)}
KICK_START, KICK_MAX = D(875950), D(66650)        # i6251 p.9, line 4

def ord_tax(ti, st):
    t=D(0); lo=D(0)
    for hi,r in BR[st]:
        if hi is None or ti<=hi: return t+(ti-lo)*r
        t+=(hi-lo)*r; lo=hi

def qdcgt(ti, pref, st):
    """Regular-tax QDCGT worksheet. Returns (tax, line5) where line5 = the ordinary bottom."""
    ti=max(ti,D(0)); pref=min(max(pref,D(0)), ti)
    l5 = ti - pref                                    # worksheet L5 -> 6251 lines 20/27
    lo=l5; hi=ti; t=D(0)
    for top,rate in ((L19[st],D(0)),(L25[st],D('.15')),(None,D('.20'))):
        seg = max(D(0),(hi-lo) if top is None else min(hi,top)-lo); t+=seg*rate
        if top is None or hi<=top: break
        lo=max(lo,top)
    return min(ord_tax(l5,st)+t, ord_tax(ti,st)), l5

def form6251(*, st, taxable_income_L15, agi_L11, deduction_L14, sch_a_line7, itemized,
             refund_sch1_L1, net_ltcg, qual_div, regular_tax_L16, sch2_L1z=D(0), sch3_L1=D(0)):
    F={}
    F[1]  = taxable_income_L15 if taxable_income_L15 > 0 else agi_L11 - deduction_L14
    F['2a'] = sch_a_line7 if itemized else STD[st]
    F['2b'] = -refund_sch1_L1
    F[3]  = D(0)
    F[4]  = F[1] + F['2a'] + F['2b'] + F[3]
    if st == 'mfs' and F[4] > KICK_START:            # i6251 p.9 line 4
        F[4] += min(D('.25')*(F[4]-KICK_START), KICK_MAX)
    F[5]  = max(D(0), EXM[st] - D('.25')*max(D(0), F[4]-PHASE[st]))
    F[6]  = F[4] - F[5]
    if F[6] <= 0:                                     # "enter -0- here and on lines 7, 9, and 11"
        F[6]=F[7]=F[9]=F[11]=D(0); F[10]=max(D(0), regular_tax_L16+sch2_L1z-sch3_L1); return F
    if net_ltcg + qual_div > 0:                       # line 7, middle bullet -> Part III
        _, l5 = qdcgt(taxable_income_L15, qual_div+net_ltcg, st)
        F[12]=F[6]; F[13]=qual_div+net_ltcg; F[14]=D(0); F[15]=F[13]
        F[16]=min(F[12],F[15]); F[17]=F[12]-F[16]
        F[18]=D('.26')*F[17] if F[17]<=BP28[st] else D('.28')*F[17]-SUB[st]
        F[19]=L19[st]; F[20]=l5; F[21]=max(D(0),F[19]-F[20])
        F[22]=min(F[12],F[13]); F[23]=min(F[21],F[22]); F[24]=F[22]-F[23]
        F[25]=L25[st]; F[26]=F[21]; F[27]=l5; F[28]=F[26]+F[27]
        F[29]=max(D(0),F[25]-F[28]); F[30]=min(F[24],F[29]); F[31]=D('.15')*F[30]
        F[32]=F[23]+F[30]
        if F[32]==F[12]: F[33]=F[34]=F[35]=F[36]=F[37]=D(0)   # line 32 skip
        else:
            F[33]=F[22]-F[32]; F[34]=D('.20')*F[33]   # line 33: 'Subtract line 32 from line 22'
            F[35]=F[36]=F[37]=D(0)                             # line 34: L14 blank -> skip 35-37
        F[38]=F[18]+F[31]+F[34]+F[37]
        F[39]=D('.26')*F[12] if F[12]<=BP28[st] else D('.28')*F[12]-SUB[st]
        F[40]=min(F[38],F[39]); F[7]=F[40]
    else:                                             # line 7, "All others"
        F[7]=D('.26')*F[6] if F[6]<=BP28[st] else D('.28')*F[6]-SUB[st]
    F[10]=max(D(0), regular_tax_L16+sch2_L1z-sch3_L1)          # computed BEFORE line 8 (i6251 p.10)
    F[8] =D(0) if F[10]>=F[7] else sch3_L1
    F[9] =F[7]-F[8]
    F[11]=max(D(0), F[9]-F[10])
    return F
