;              T         T               T

; Compact demo-startup by Blueberry/Loonies
; Version 1.5, October 7, 2014
; Public Domain

;; ********** System constants **********

; Exec
SU_Supervisor = -30
SU_OldOpenLibrary = -408
SU_CacheClearU = -636
SU_TaggedOpenLibrary = -810
; Graphics
SU_OpenFont = -72
SU_LoadView = -222
SU_WaitTOF = -270

SU_lib_Version = $14
SU_eb_AttnFlags = $128
SU_gb_ActiView = $22
SU_gb_copinit = $26
SU_tf_CharData = $22
IVBLIT = 156


; Cache flush macro
; Potentially trashes D0-D1/A0-A1/A6

	if	COMPATIBILITY=0
CACHEFLUSH	macro
	endm
	endc

	if	COMPATIBILITY=1
CACHEFLUSH	macro
	move.l	$4.w,a6
	cmp.w	#37,SU_lib_Version(a6)
	blo.b	.noflush\@
	jsr	SU_CacheClearU(a6)
.noflush\@:
	endm
	endc

	if	COMPATIBILITY=2
CACHEFLUSH	macro
	move.l	$4.w,a6
	jsr	SU_CacheClearU(a6)
	endm
	endc

DMA_OFF	=	$00ff-(COPPER<<7)-(BLITTER<<6)-(SPRITE<<5)


; Entry point

	if	FASTMEM
	section	Code,code_f
	else
	section	Code,code
	endc

Startup:
	if	SECTIONHACK<>0
	if	SECTIONHACK=1
	; Read section pointers from hunk structure
	move.l	Startup-4(pc),d0
	lsl.l	#2,d0
	addq.l	#4,d0
	lea.l	ChipPtr(pc),a1
	move.l	d0,(a1)
	endc
	else
	move.l	#Chip,ChipPtr
	endc

	; Open topaz
	if	TOPAZ
	move.l	$4.w,a6
	move.l	156(a6),a6	; graphics.library
	lea.l	TextAttr(pc),a0
	lea.l	TopazName(pc),a4
	move.l	a4,(a0)
	jsr	SU_OpenFont(a6)
	move.l	d0,a0
	move.l	SU_tf_CharData(a0),TopazCharData-TopazName(a4)
	endc

	; Call precalc code
	bsr.w	_Precalc

	; Retrieve Vector Base Register
	move.l	$4.w,a6
	if	COMPATIBILITY=1
	suba.l	a2,a2
	btst.b	#0,SU_eb_AttnFlags+1(a6)
	beq.b	.no010
	endc
	if	COMPATIBILITY>=1
	lea.l	GetVBR(pc),a5
	jsr	SU_Supervisor(a6)
.no010:	endc

	; Clear display
	move.l	156(a6),a6	; graphics.library
	if	COMPATIBILITY>=1
	move.l	SU_gb_ActiView(a6),d7
	suba.l	a1,a1
	jsr	SU_LoadView(a6)
	lea.l	$dff000,a3
	move.w	#$0020,$1dc(a3)	; Ensure PAL
	jsr	SU_WaitTOF(a6)
	jsr	SU_WaitTOF(a6)
	else
	jsr	SU_WaitTOF(a6)
	lea.l	$dff000,a3
	endc
	move.w	#$3fdf,$09a(a3)
	move.w	#DMA_OFF,$096(a3)

	; Init interrupt
	lea.l	InterruptHandler(pc),a4
	if	COMPATIBILITY>=1
	move.l	$6c(a2),a5
	move.l	a4,$6c(a2)
	movem.l	d7/a2/a3/a5/a6,-(a7)
	else
	move.l	$6c.w,a5
	move.l	a4,$6c.w
	movem.l	a3/a5/a6,-(a7)
	endc
	move.l	a7,-(a4)	; StackStore

	bsr.w	_Main

StartupEnd:
	move.l	StackStore(pc),a7

	; Restore interrupt
	if	COMPATIBILITY>=1
	movem.l	(a7)+,a1/a2/a3/a4/a6
	move.l	a4,$6c(a2)
	else
	movem.l	(a7)+,a3/a4/a6
	move.l	a4,$6c.w
	endc

	; Restore display
	move.w	#$a00c,$09a(a3)
	move.w	#$001f,$096(a3)
	move.w	#$81e0,$096(a3)
	if	COPPER
	move.l	SU_gb_copinit(a6),$080(a3)
	endc
	if	COMPATIBILITY>=1
	jsr	SU_LoadView(a6)
	endc

; In principle, we should close graphics.library and topaz.font here,
; but since they are always in use, it makes no difference whatsoever,
; so we don't bother.

	bra.w	_Exit


	if	COMPATIBILITY>=1
GetVBR:
	movec	vbr,a2
	rte
	endc

	if	TOPAZ
TextAttr:
	dc.l	0
	dc.w	8
	dc.b	0,0
TopazCharData:	dc.l	0
TopazName:	dc.b	'topaz.font',0
	endc

	even

VBlank:	dc.l	0
	if	SECTIONHACK>=0
ChipPtr:	dc.l	0
	endc
StackStore:	dc.l	0
InterruptHandler:
	movem.l	d0-a6,-(a7)
	if	FPUINT
	fsave	-(a7)
	fmovem.x	fp0-fp7,-(a7)
	endc

	lea.l	VBlank(pc),a0
	addq.l	#1,(a0)

	bsr.w	_Interrupt
	lea.l	$dff000,a3

	if	RMBPAUSE
	; RMB pause
.rmb:	btst	#10,$016(a3)
	bne.b	.normb
	move.w	#$0020,$09c(a3)
	move.w	#$0020,$09c(a3)

	btst.b	#6,$bfe001
	bne.b	.rmbloop2
.rmbloop:
	moveq.l	#$0020,d0
	and.w	$dff01e,d0
	beq.b	.rmbloop
	bsr.w	_Interrupt
	lea.l	$dff000,a3
 	bra.b	.rmb
.rmbloop2:
	moveq.l	#$0020,d0
	and.w	$dff01e,d0
	beq.b	.rmbloop2
	bsr.w	_Interrupt
	lea.l	$dff000,a3
	btst.b	#6,$bfe001
	bne.b	.rmb
	move.w	#$0020,$09c(a3)
	move.w	#$0020,$09c(a3)
	if	FPUINT
	fmovem.x	(a7)+,fp0-fp7
	frestore	(a7)+
	endc
	movem.l	(a7)+,d0-a6
	rte
.normb:
	endc

	move.w	#$0020,$09c(a3)
	if	COMPATIBILITY>=1
	move.w	#$0020,$09c(a3)
	endc

	if	FPUINT
	fmovem.x	(a7)+,fp0-fp7
	frestore	(a7)+
	endc
	movem.l	(a7)+,d0-a6

	btst.b	#6,$bfe001
	bne.b	.end
	lea.l	StartupEnd(pc),a1
	move.l	a1,2(a7)
.end:
	rte
