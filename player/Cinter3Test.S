;              T         T               T

COMPATIBILITY	=	1
FASTMEM	=	0
RMBPAUSE	=	0
FPUINT	=	0
COPPER	=	0
BLITTER	=	0
SPRITE	=	0
TOPAZ	=	0
SECTIONHACK	=	0

	include	Cinter:DemoStartup.S
	include	Cinter:Cinter3.S

_Exit:
	moveq.l	#0,d0
	rts

_Precalc:
	move.l	ChipPtr(pc),a4
	add.l	#InstrumentSpace-Chip,a4
	lea	MusicData(pc),a2
	move.l	a2,a6
	add.l	#CinterSpace-MusicData,a6

	; Copy raw instrument data to instrument space
	move.l	a2,a0
	add.l	#InstrumentData-MusicData,a0
	move.l	a4,a1
	move.l	#(InstrumentData_End-InstrumentData)/2,d7
.copy:	move.w	(a0)+,(a1)+
	subq.l	#1,d7
	bgt.b	.copy

	; A2 = Music data
	; A4 = Instrument space
	; A6 = Cinter working memory
	bsr.w	CinterInit
	rts

_Main:

MainLoop:
	bra.w	MainLoop

_Interrupt:
	lea	MusicData(pc),a2
	move.l	a2,a6
	add.l	#CinterSpace-MusicData,a6
	; A6 = Cinter working memory
	bsr.w	CinterPlay1

	; Put your other interrupt code here

	lea	MusicData(pc),a2
	move.l	a2,a6
	add.l	#CinterSpace-MusicData,a6
	; A6 = Cinter working memory
	bsr.w	CinterPlay2
	rts

MusicData:
	incbin	Cinter:automatic.cinter3

InstrumentData:
	incbin	Cinter:automatic.raw
InstrumentData_End:


CinterSpace:
	ds.b	c_SIZE


	section	inst,bss_c
Chip:

InstrumentSpace:
	ds.b	200000
