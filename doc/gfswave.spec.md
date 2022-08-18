https://polar.ncep.noaa.gov/mmab/papers/tn222/MMAB_222.pdf

Page 72

### Wave Partitioning from `w3partmd`

https://github.com/NOAA-EMC/WW3/blob/develop/model/src/w3partmd.F90

```f90
      USE W3GDATMD, ONLY: NK, NTH, NSPEC, SIG, TH
      USE W3ODATMD, ONLY: WSCUT, FLCOMB
!
      IMPLICIT NONE
!/
!/ ------------------------------------------------------------------- /
!/ Parameter list
!/
      INTEGER, INTENT(OUT)          :: NP
      INTEGER, INTENT(IN)           :: DIMXP
      REAL, INTENT(IN)              :: SPEC(NK,NTH), WN(NK), UABS,    &
                                       UDIR, DEPTH
      REAL, INTENT(OUT)             :: XP(DIMP,0:DIMXP)
```

`NK`: number of frequencies (wave number: `k`)
`NTH`: number of directions (`TH`: theta RAD)
`SPEC`: energy for each freq, direction combo
`WN`: Wave number for each frequency
`UABS`: wind speed
`UDIR`: wind direction
`SIG`: !  Relative frequencies (invariant in grid). (rad)
`WSMULT`: Multiplier for wind sea boundary

`SIG` from w3gridmd.f90

```f90
      XFR    = MAX ( RXFR , 1.00001 ) // commonly 1.1 can be calculated on the fly
      FR1    = MAX ( RFR1 , 1.E-6 ) // lowest frq, we know this
      ....
      SIGMA   = FR1 * TPI / XFR**2
      SXFR    = 0.5 * (XFR-1./XFR)
!
      DO IK=0, NK+1
        SIGMA    = SIGMA * XFR
        SIG (IK) = SIGMA
        DSIP(IK) = SIGMA * SXFR
        END DO
```

```f90 
!
! -------------------------------------------------------------------- /
! 1.  Process input spectrum
! 1.a 2-D to 1-D spectrum
!
      DO ITH=1, NTH
        ZP(1+(ITH-1)*NK:ITH*NK) = SPEC(:,ITH)
        END DO
```

zp = array[freq*direction]
For direction: 
    zp[direction_index * freq_count] = specs[:, dir]

The flattened array dumps all the energies for each direction into a 1d array

From here, the users input shows the different way to calculate the wave partitions

```f90
!
! PTMETH == 4 : Do simple partitioning based solely on the
! wave age criterion (produces one swell and one wind sea only):
!
      IF( PTMETH .EQ. 4 ) THEN
        DO IK=1, NK
          DO ITH=1, NTH
             ISP = IK + (ITH-1) * NK ! index into partition array IMO

             UPAR = WSMULT * UABS * MAX(0.0, COS(TH(ITH)-DERA*UDIR))
             C = SIG(IK) / WN(IK)

             IF( UPAR .LE. C ) THEN
               ! Is swell:
               IMO(ISP) = 2
             ELSE
               ! Is wind sea:
               IMO(ISP) = 1
             ENDIF
          ENDDO
        ENDDO

        ! We have a max of up to two partitions:
        NP_MAX=2

        ! Calculate mean parameters:
        CALL PTMEAN ( NP_MAX, IMO, ZP, DEPTH, UABS, UDIR, WN,           &
                    NP, XP, DIMXP, PMAP )

        ! No more processing required, return:
        RETURN
      ENDIF ! PTMETH == 4
!
```

for each frequency, loop over directions, then sorts each dirction+frequency component as swell or wind partition

then calls PTMEAN to calculate params... 

#### PTMEAN

```
!> @brief Compute mean parameters per partition
!>
!> @param[in]    NPI     Number of partitions found.
!> @param[in]    IMO     Partition map.
!> @param[in]    ZP      Input spectrum.
!> @param[in]    DEPTH   Water depth.
!> @param[in]    UABS    Wind speed.
!> @param[in]    UDIR    Wind direction.
!> @param[in]    WN      Wavenumebers for each frequency.
!> @param[out]   NPO     Number of partitions with mean parameters.
!> @param[out]   XP      Array with output parameters.
!> @param[in]    DIMXP   Second dimension of XP.
!> @param[out]   PMAP    Mapping between orig. and combined partitions
!>
!> @author Barbara Tracy, H. L. Tolman, M. Szyszka, C. Bunney
!> @date 02 Dec 2010
!>
      SUBROUTINE PTMEAN ( NPI, IMO, ZP, DEPTH, UABS, UDIR, WN,        &
                          NPO, XP, DIMXP, PMAP )
```

```f90
      DO IK=1, NK
        DO ITH=1, NTH
          ISP    = IK + (ITH-1)*NK
          IP     = IMO(ISP)
          FACT   = MAX ( 0. , MIN ( 1. ,                              &
            1. - ( FCDIR(ITH) - 0.5*(SIG(IK-1)+SIG(IK)) ) / DSIP(IK) ) )
          SUMF (IK, 0) = SUMF (IK, 0) + ZP(ISP)
          SUMFW(IK, 0) = SUMFW(IK, 0) + ZP(ISP) * FACT
          SUMFX(IK, 0) = SUMFX(IK, 0) + ZP(ISP) * ECOS(ITH)
          SUMFY(IK, 0) = SUMFY(IK, 0) + ZP(ISP) * ESIN(ITH)
          IF ( IP .EQ. 0 ) CYCLE
          SUMF (IK,IP) = SUMF (IK,IP) + ZP(ISP)
          SUMFW(IK,IP) = SUMFW(IK,IP) + ZP(ISP) * FACT
          SUMFX(IK,IP) = SUMFX(IK,IP) + ZP(ISP) * ECOS(ITH)
          SUMFY(IK,IP) = SUMFY(IK,IP) + ZP(ISP) * ESIN(ITH)
          END DO
        END DO
      SUMF(NK+1,:) = SUMF(NK,:) * FACHFE
```

for each frequency and direction, it gets the partition number and computes the integral for each partition

```f90
      DO IP=0, NPI
        DO IK=1, NK
          SUME (IP) = SUME (IP) + SUMF (IK,IP) * DSII(IK)
          SUMQP(IP) = SUMQP(IP) + SUMF (IK,IP)**2 * DSII(IK) * SIG(IK)
          SUME1(IP) = SUME1(IP) + SUMF (IK,IP) * DSII(IK) * SIG(IK)
          SUME2(IP) = SUME2(IP) + SUMF (IK,IP) * DSII(IK) * SIG(IK)**2
          SUMEM1(IP) = SUMEM1(IP) + SUMF (IK,IP) * DSII(IK) / SIG(IK)

          SUMEW(IP) = SUMEW(IP) + SUMFW(IK,IP) * DSII(IK)
          SUMEX(IP) = SUMEX(IP) + SUMFX(IK,IP) * DSII(IK)
          SUMEY(IP) = SUMEY(IP) + SUMFY(IK,IP) * DSII(IK)
          IF ( SUMF(IK,IP) .GT. EFPMAX(IP) ) THEN
              IFPMAX(IP) = IK
              EFPMAX(IP) = SUMF(IK,IP)
            END IF
          END DO

        !SUME (IP) = SUME (IP) + SUMF (NK,IP) * FTE
        !SUME1(IP) = SUME1(IP) + SUMF (NK,IP) * FTE
        !SUME2(IP) = SUME2(IP) + SUMF (NK,IP) * FTE
        !SUMEM1(IP) = SUMEM1(IP) + SUMF (NK,IP) * FTE
        !SUMQP(IP) = SUMQP(IP) + SUMF (NK,IP) * FTE
        !SUMEW(IP) = SUMEW(IP) + SUMFW(NK,IP) * FTE
        !SUMEX(IP) = SUMEX(IP) + SUMFX(NK,IP) * FTE
        !SUMEY(IP) = SUMEY(IP) + SUMFY(NK,IP) * FTE
        ! Met Office: Proposed bugfix for tail calculations, previously
        !  PT1 and PT2 values were found to be too low when using the
        !  FTE scaling factor for the tail. I think there are two issues:
        !  1. energy spectrum is scaled in radian frequency space above by DSII.
        !     This needs to be consistent and FTE contains a DTH*SIG(NK)
        !     factor that is not used in the DSII scaled calcs above
        !  2. the tail fit calcs for period parameters needs to follow
        !     the form used in w3iogomd and scaling should be
        !     based on the relationship between FTE and FT1, FTTR etc.
        !     as per w3iogomd and ww3_grid
        FTEII = FTE / (DTH * SIG(NK)) 
        SUME (IP) = SUME (IP) + SUMF (NK,IP) * FTEII
        SUME1(IP) = SUME1(IP) + SUMF (NK,IP) * SIG(NK) * FTEII * (0.3333 / 0.25)
        SUME2(IP) = SUME2(IP) + SUMF (NK,IP) * SIG(NK)**2 * FTEII * (0.5 / 0.25)
        SUMEM1(IP) = SUMEM1(IP) + SUMF (NK,IP) / SIG(NK) * FTEII * (0.2 / 0.25)
        SUMQP(IP) = SUMQP(IP) + SUMF (NK,IP) * FTEII
        SUMEW(IP) = SUMEW(IP) + SUMFW(NK,IP) * FTEII
        SUMEX(IP) = SUMEX(IP) + SUMFX(NK,IP) * FTEII
        SUMEY(IP) = SUMEY(IP) + SUMFY(NK,IP) * FTEII

        END DO
```

then for each partition, it computes the spectral moment params 

```f90
     DO IP=0, NPI
!
        SUMEXP = 0.
        SUMEYP = 0.
!
        M0 = SUME(IP)  * DTH * TPIINV
        HS     = 4. * SQRT ( MAX( M0 , 0. ) )
        IF ( HS .LT. HSPMIN ) THEN
          ! For wind cutoff and 2-band partitioning methods, keep the 
          ! partition, but set the integrated parameters to UNDEF
          ! for Hs values less that HSPMIN:
          IF( PTMETH .EQ. 4 .OR. PTMETH .EQ. 5 ) THEN
             NPO = NPO + 1
             XP(:,NPO) = UNDEF
             XP(6,NPO) = 0.0 ! Set wind sea frac to zero
          ENDIF
          CYCLE
        ENDIF
!
        IF ( NPO .GE. DIMXP ) GOTO 2000
        NPO = NPO + 1
        IF (IP.GT.0)THEN
           IF(NPO.LT.1)CYCLE
           PMAP(NPO) = IP
        ENDIF
!
        M1 = SUME1(IP) * DTH * TPIINV**2
        M2 = SUME2(IP) * DTH * TPIINV**3
        MM1 = SUMEM1(IP) * DTH
        QP = SUMQP(IP) *(DTH * TPIINV)**2
!       M1 = MAX( M1, 1.E-7 )
!       M2 = MAX( M2, 1.E-7 )
!
        XL     = 1. / XFR - 1.
        XH     = XFR - 1.
        XL2    = XL**2
        XH2    = XH**2
        EL     = SUMF(IFPMAX(IP)-1,IP) - SUMF(IFPMAX(IP),IP)
        EH     = SUMF(IFPMAX(IP)+1,IP) - SUMF(IFPMAX(IP),IP)
        DENOM  = XL*EH - XH*EL
        SIGP   = SIG(IFPMAX(IP))
```

then it computes the raw parameters of the waves 

```f90
     !/ --- Parabolic fit around the spectral peak ---
        IK = IFPMAX(IP)
        EFPMAX(IP) = SUMF(IK,IP) * DTH
        IF (IK.GT.1 .AND. IK.LT.NK) THEN
          EL    = SUMF(IK-1,IP) * DTH
          EH    = SUMF(IK+1,IP) * DTH
          NUMER = 0.125 * ( EL - EH )**2
          DENOM = EL - 2. * EFPMAX(IP) + EH
          IF (DENOM.NE.0.) EFPMAX(IP) = EFPMAX(IP)         &
                          - NUMER / SIGN( ABS(DENOM),DENOM )
        END IF
!
     !/ --- Weighted least-squares regression to estimate frequency
     !/     spread (FSPRD) to an exponential function:
     !/              E(f) = A * exp(-1/2*(f-fp)/B)**2             ,
     !/     where B is frequency spread and  E(f) is used for
     !/     weighting to avoid greater weights to smalll values
     !/     in ordinary least-square fit. ---
        FSPRD     = UNDEF
        SUMY      = 0.
        SUMXY     = 0.
        SUMXXY    = 0.
        SUMYLOGY  = 0.
        SUMXYLOGY = 0.
!
        DO IK=1, NK
          Y = SUMF(IK,IP)*DTH
        ! --- sums for weighted least-squares ---
          IF (Y.GE.1.E-15) THEN
            YHAT = LOG(Y)
            XHAT = -0.5 * ( (SIG(IK)-SIGP)*TPIINV )**2
            SUMY      = SUMY + Y
            SUMXY     = SUMXY + XHAT * YHAT
            SUMXXY    = SUMXXY + XHAT * XHAT * Y
            SUMYLOGY  = SUMYLOGY + Y * YHAT
            SUMXYLOGY = SUMXYLOGY + SUMXY * YHAT
          END IF
        END DO
!
        NUMER = SUMY * SUMXXY - SUMXY**2
        DENOM = SUMY * SUMXYLOGY - SUMXY * SUMYLOGY
        IF (DENOM.NE.0.)  FSPRD = SQRT( NUMER / SIGN(ABS(DENOM),NUMER) )
!
        SUMEXP = SUMFX(IFPMAX(IP),IP) * DSII(IFPMAX(IP))
        SUMEYP = SUMFY(IFPMAX(IP),IP) * DSII(IFPMAX(IP))
```

Then it uses a parabolic fit to increase accuracy

```f90
     !/ --- Significant wave height ---
        XP(1,NPO) = HS
     !/ --- Peak wave period ---
        XP(2,NPO) = TPI / SIGP
     !/ --- Peak wave length ---
        XP(3,NPO) = TPI / WNP
     !/ --- Mean wave direction ---
        XP(4,NPO) = MOD( 630.-ATAN2(SUMEY(IP),SUMEX(IP))*RADE , 360. )
     !/ --- Mean directional spread ---
        XP(5,NPO) = RADE * SQRT ( MAX ( 0. , 2. * ( 1. - SQRT ( &
                MAX(0.,(SUMEX(IP)**2+SUMEY(IP)**2)/SUME(IP)**2) ) ) ) )
     !/ --- Wind sea fraction ---
        XP(6,NPO) = SUMEW(IP) / SUME(IP)
     !/ --- Peak wave direction ---
        XP(7,NPO) =  MOD(630.-ATAN2(SUMEYP,SUMEXP)*RADE , 360.)
     !/ --- Spectral width (Longuet-Higgins 1975) ---
        XP(8,NPO) = SQRT( MAX( 1. , M2*M0 / M1**2 ) - 1. )
     !/ --- JONSWAP peak enhancement parameter (E(fp)/EPM(fp))---
     !  EPM_FMX = ALPHA_PM_FMX * GRAV**2 * TPI * SIGP**-5 * EXP(-5/4)
        ALP_PM = 0.3125 * HS**2 * (SIGP)**4
        EPM_FP = ALP_PM * TPI * (SIGP**(-5)) * 2.865048E-1
        XP(9,NPO) = MAX( EFPMAX(IP) / EPM_FP , 1.0 )
     !/ --- peakedness parameter (Goda 1970) ---
        XP(10,NPO) = 2. * QP / M0**2
     !/ --- gaussian frequency width ---
        XP(11,NPO) = FSPRD
     !/ --- wave energy period (inverse moment) ---
        XP(12,NPO) = MM1 / M0
     !/ --- mean wave period (first moment) ---
        XP(13,NPO) = M0 / M1
     !/ --- zero-upcrossing period (second moment) ---
        XP(14,NPO) = SQRT( M0 / M2 )
     !/ --- peak spectral density (one-dimensional) ---
        XP(15,NPO) = EFPMAX(IP)
```

and finally it computes the output params for each partition.. 


Looking more closely at calculating `HS` (significant wave height):

```f90
       M0 = SUME(IP)  * DTH * TPIINV
        HS     = 4. * SQRT ( MAX( M0 , 0. ) )
```

leads us to 

```f90
SUME (IP) = SUME (IP) + SUMF (IK,IP) * DSII(IK)
```

then

```f90
SUMF (IK, 0) = SUMF (IK, 0) + ZP(ISP)
```

so `SUMF` is the sum of all energy components for a given freq. 

`DTH` is:  `DTH       Real  Public   Directional increments (radians).`
`DSII` is: `DSII      R.A   Public   Frequency bandwidths (int.)     (rad)`
`TPIINV`: `TPIINV = 1. / TPI !< TPIINV Inverse of 2*Pi.`

easy enough... now for period: 

```f90
     !/ --- Peak wave period ---
        XP(2,NPO) = TPI / SIGP
```

`TPI`: `REAL, PARAMETER :: TPI = 2.0 * PI !< TPI 2*Pi.`

```f90
SIGP   = SIG(IFPMAX(IP))
```

`SIG`: `SIG: !  Relative frequencies (invariant in grid). (rad)`

```f90
          IF ( SUMF(IK,IP) .GT. EFPMAX(IP) ) THEN
              IFPMAX(IP) = IK
              EFPMAX(IP) = SUMF(IK,IP)
            END IF
          END DO
```

So that is just the frequency with the maximum period for a given partition

OK... Now for direction: 

```f90
     !/ --- Mean wave direction ---
        XP(4,NPO) = MOD( 630.-ATAN2(SUMEY(IP),SUMEX(IP))*RADE , 360. )

     !/ --- Peak wave direction ---
        XP(7,NPO) =  MOD(630.-ATAN2(SUMEYP,SUMEXP)*RADE , 360.)
```

first `SUMEY`

```f90
SUMEY(IP) = SUMEY(IP) + SUMFY(IK,IP) * DSII(IK)
... 
FTEII = FTE / (DTH * SIG(NK)) 
SUMEY(IP) = SUMEY(IP) + SUMFY(NK,IP) * FTEII
```
`FTE`:       `Real  Public   Factor in tail integration energy.`

from `w3iogomod.f09`
```f90
! FTE    = 0.25 * SIG(NK) * DTH * SIG(NK) [ww3_grid.ftn]
```

We know `DSII`, how about `SUMFY`: 

```f90
SUMFY(IK, 0) = SUMFY(IK, 0) + ZP(ISP) * ESIN(ITH)
```

where `ESIN`: `Sine of discrete directions.`

ok, now `SUMEX`

```f90
SUMEX(IP) = SUMEX(IP) + SUMFX(IK,IP) * DSII(IK)
...
SUMEX(IP) = SUMEX(IP) + SUMFX(NK,IP) * FTEII
```

and `SUMFX`

```f90
SUMFX(IK, 0) = SUMFX(IK, 0) + ZP(ISP) * ECOS(ITH)
```

where `ECOS`: `Cosine of discrete directions.`

lastly `SUMEYP` and `SUMEXP`: 

```f90
        SUMEXP = SUMFX(IFPMAX(IP),IP) * DSII(IFPMAX(IP))
        SUMEYP = SUMFY(IFPMAX(IP),IP) * DSII(IFPMAX(IP))
```

where `IFPMAX` is the index of the peak period (Freq)