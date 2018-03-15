;              T         T               T

COMPATIBILITY	=	1
FASTMEM	=	0
RMBPAUSE	=	0
FPUINT	=	0
COPPER	=	1
BLITTER	=	0
SPRITE	=	0
TOPAZ	=	0
SECTIONHACK	=	0

CINTER_MANUAL_DMA =	0

	include	Cinter:DemoStartup.S
	include	Cinter:Cinter4.S

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
	if	CINTER_MANUAL_DMA
	move.l	ChipPtr(pc),a4
	add.l	#Copper-Chip,a4
	move.l	a4,$dff080
	move.l	#$1001fffe,(a4)+
	move.l	#$00960000,(a4)+
	move.l	#$fffffffe,(a4)+
	endc

MainLoop:
	bra.w	MainLoop

_Interrupt:
	lea	MusicData(pc),a2
	move.l	a2,a6
	add.l	#CinterSpace-MusicData,a6
	; A6 = Cinter working memory
	bsr.w	CinterPlay1

	; Manual writing of DMA enable
	if	CINTER_MANUAL_DMA
	move.l	ChipPtr(pc),a4
	add.l	#CopperDMA-Chip,a4
	or.w	#$8000,d0
	move.w	d0,(a4)
	endc

	; Put your other interrupt code here

	lea	MusicData(pc),a2
	move.l	a2,a6
	add.l	#CinterSpace-MusicData,a6
	; A6 = Cinter working memory
	bsr.w	CinterPlay2
	rts

MusicData:
	incbin	Cinter:automatic.cinter4

InstrumentData:
	incbin	Cinter:automatic.raw
InstrumentData_End:


CinterSpace:
	ds.b	c_SIZE


	section	inst,bss_c
Chip:

	if	CINTER_MANUAL_DMA
Copper:
	ds.l	3
CopperDMA	=	Copper+6
	endc

InstrumentSpace:
	ds.b	200000
